use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{self, Path, PathBuf},
    sync::Arc,
};

use tokio::{
    sync::{Mutex, OnceCell},
    task::JoinSet,
};
use unicase::UniCase;

use super::controller_impl::Controller;
use super::internal::{
    exists_mod::ExistsMod,
    file_dpac::parse_dpac_file,
    file_internal::FileInternal,
    file_task::parse_task_file,
    file_text::parse_text_file,
    load_mdl::{LoadMdl, LoadMdlItem},
    tcp_ip_settings::TcpIpSettings,
    util::read_file_cp850,
};

type Cache = Arc<Mutex<HashMap<PathBuf, Arc<OnceCell<Option<Arc<FileInternal>>>>>>>;

const Q_SYSTEM: [&str; 7] = [
    "SLib:QSystem.Dpe",
    "SLib:QCom.Dpe",
    "SLib:QDisp.Dpe",
    "SLib:QServices.Dpe",
    "SLib:QDig.Dpe",
    "SLib:QAnaIn.Dpe",
    "SLib:QAnaOut.Dpe",
];

/// Loader for [Controller].
///
/// DPac's in `prod_dir/SLib` will be cached and reused.
pub struct ControllerLoader {
    mode: LoadMode,
    prod_dir: Option<PathBuf>,
    cache: Cache,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LoadMode {
    /// Hashed names consumes the least amount of memory but iteration over variables in files is not useful.
    /// It's still possible to lookup variables by name.
    HashedNames,
    /// Consumes more memory but gives names when iteration over variables in files.
    WithNames,
    /// Like [LoadMode::WithNames] but also with comments. Uses the most memory.
    WithNamesAndComments,
}

impl ControllerLoader {
    /// Creates a new loader with default configuration.
    /// Prod dir is required for loading system DPac's and prefab controllers.
    pub fn new(prod_dir: Option<PathBuf>) -> Self {
        Self::new_with_mode(prod_dir, LoadMode::HashedNames)
    }

    /// Creates a new loader with specified configuration.
    /// Prod dir is required for loading system DPac's and prefab controllers.
    pub fn new_with_mode(prod_dir: Option<PathBuf>, mode: LoadMode) -> Self {
        Self {
            mode,
            prod_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl ControllerLoader {
    /// Load everything. Which can use a lot of memory.
    pub async fn load_all(&self, controller_dir: &Path) -> std::io::Result<Controller> {
        self.load_selective(controller_dir, |_, _| true).await
    }

    /// Load globals (and system DPac's). Usually significantly less to load.
    pub async fn load_globals(&self, controller_dir: &Path) -> std::io::Result<Controller> {
        self.load_selective(controller_dir, |filename, global| {
            if global {
                return true;
            }
            Q_SYSTEM.contains(&filename)
        })
        .await
    }

    /// Select what files to load.
    /// The callback takes filename and global flag arguments.
    pub async fn load_selective<S>(&self, controller_dir: &Path, mut selector: S) -> std::io::Result<Controller>
    where
        S: FnMut(&str, bool) -> bool,
    {
        let exists_mod_content = read_file_cp850(&controller_dir.join("Exists.Mod")).await?;
        let exists_mod = ExistsMod::parse(&exists_mod_content);

        let module_library_dir = match exists_mod.module_library {
            None => controller_dir.into(),
            Some(module_library) => match self.resolve_filename(&module_library, controller_dir, None) {
                None => controller_dir.into(),
                Some(module_library_dir) => module_library_dir,
            },
        };

        let load_mdl_content = read_file_cp850(&module_library_dir.join("Load.Mdl")).await?;
        let mut load_mdl = LoadMdl::parse(&load_mdl_content);

        // Add system DPac's
        for filename in Q_SYSTEM {
            load_mdl.dpacs.push(LoadMdlItem {
                filename: filename.into(),
                global: false,
                load_number: None,
            });
        }

        let mut load_set = JoinSet::new();

        let tasks_iter = load_mdl.tasks.iter().enumerate().map(|i| (LoadFileKind::Task, i.0, i.1));
        let dpacs_iter = load_mdl.dpacs.iter().enumerate().map(|i| (LoadFileKind::DPac, i.0, i.1));
        let texts_iter = load_mdl.texts.iter().enumerate().map(|i| (LoadFileKind::Text, i.0, i.1));

        let slib_dir = Arc::new(self.prod_dir.as_ref().map(|p| p.join("SLib")));

        for (kind, i, item) in tasks_iter.chain(dpacs_iter).chain(texts_iter) {
            if !selector(&item.filename, item.global) {
                continue;
            }
            let path = match self.resolve_filename(&item.filename, controller_dir, Some(module_library_dir.as_path())) {
                None => continue,
                Some(path) => path,
            };
            let path = controller_dir.join(path);
            let cache = self.cache.clone();
            let slib_dir = slib_dir.clone();
            let mode = self.mode;
            load_set.spawn(async move {
                let file = load_file(cache, &slib_dir, kind, &path, mode).await;
                (kind, i, path, file)
            });
        }

        let mut tasks = HashMap::with_capacity(load_mdl.tasks.len());
        let mut dpacs = HashMap::with_capacity(load_mdl.dpacs.len());
        let mut texts = HashMap::with_capacity(load_mdl.texts.len());
        let mut globals = HashMap::new();

        let tcp_ip_settings = match read_file_cp850(&controller_dir.join("TcpIpSettings.Exo")).await {
            Ok(content) => Some(TcpIpSettings::parse(&content)),
            Err(_) => None,
        };

        while let Some(content) = load_set.join_next().await {
            let (kind, i, path, file) = match content {
                Ok((kind, i, path, Some(file))) => (kind, i, path, file),
                _ => continue, // Ignore errors for individual files
            };
            let (load_mdl_item, file_set) = match kind {
                LoadFileKind::Task => (&load_mdl.tasks[i], &mut tasks),
                LoadFileKind::DPac => (&load_mdl.dpacs[i], &mut dpacs),
                LoadFileKind::Text => (&load_mdl.texts[i], &mut texts),
            };

            let name: Arc<UniCase<String>> = Arc::new(path.with_extension("").file_name().unwrap().to_string_lossy().into());

            let load_number = match kind {
                LoadFileKind::Text => 127,
                _ => match load_mdl_item.load_number.or(file.load_number) {
                    None => continue,
                    Some(load_number) => load_number,
                },
            };

            if load_mdl_item.global && matches!(kind, LoadFileKind::DPac) {
                globals.insert(name.clone(), (load_number, file.clone()));
            }
            file_set.insert(name, (load_number, file));
        }

        Ok(Controller {
            tasks: tasks.into(),
            dpacs: dpacs.into(),
            texts: texts.into(),
            globals: globals.into(),
            address: (exists_mod.pla, exists_mod.ela),
            require_password: tcp_ip_settings.as_ref().map(|s| s.require_password).unwrap_or(false),
            system_password: tcp_ip_settings.and_then(|s| s.system_password),
        })
    }

    /// Loads system DPac's only.
    /// If no files could be loaded from the `prod_dir` it will be completely empty.
    pub async fn load_system(&self) -> Controller {
        let mut load_set = JoinSet::new();

        let slib_dir = Arc::new(self.prod_dir.as_ref().map(|p| p.join("SLib")));

        for filename in Q_SYSTEM {
            let path = match self.resolve_filename(filename, Path::new(""), None) {
                None => continue,
                Some(path) => path,
            };
            let cache = self.cache.clone();
            let slib_dir = slib_dir.clone();
            let mode = self.mode;
            load_set.spawn(async move {
                let file = load_file(cache, &slib_dir, LoadFileKind::DPac, &path, mode).await;
                (filename, file)
            });
        }

        let mut dpacs = HashMap::with_capacity(Q_SYSTEM.len());

        while let Some(content) = load_set.join_next().await {
            let (filename, file) = match content {
                Ok((filename, Some(file))) => (filename, file),
                _ => continue, // Ignore errors for individual files
            };

            let name: Arc<UniCase<String>> = Arc::new(filename[5..filename.len() - 4].to_string().into());

            let load_number = match file.load_number {
                None => continue,
                Some(load_number) => load_number,
            };

            dpacs.insert(name, (load_number, file));
        }

        Controller {
            tasks: HashMap::new().into(),
            dpacs: dpacs.into(),
            texts: HashMap::new().into(),
            globals: HashMap::new().into(),
            address: (254, 254),
            require_password: false,
            system_password: None,
        }
    }

    fn resolve_filename(&self, filename: &str, controller_dir: &Path, module_library_dir: Option<&Path>) -> Option<PathBuf> {
        let (part1, part2) = match filename.split_once(':') {
            None => return Some(filename.into()),
            Some(value) => value,
        };
        let resolved_path = match part1.to_ascii_lowercase().as_str() {
            "al" | "alib" => self
                .prod_dir
                .as_ref()
                .and_then(|prod_dir| path::absolute(prod_dir.join("ALib").join(part2)).ok()),
            "sl" | "slib" => self
                .prod_dir
                .as_ref()
                .and_then(|prod_dir| path::absolute(prod_dir.join("SLib").join(part2)).ok()),
            "ml" | "mlib" => path::absolute(module_library_dir.unwrap_or(controller_dir).join(part2)).ok(),
            "proj" => path::absolute(controller_dir.join("..").join(part2)).ok(),
            "prod" => self.prod_dir.as_ref().and_then(|prod_dir| path::absolute(prod_dir.join(part2)).ok()),
            _ => Some(filename.into()),
        };

        Some(resolved_path.unwrap_or_else(|| filename.into()))
    }
}

#[derive(Clone, Copy)]
enum LoadFileKind {
    Task,
    DPac,
    Text,
}

async fn load_file(cache: Cache, slib_dir: &Option<PathBuf>, kind: LoadFileKind, path: &Path, mode: LoadMode) -> Option<Arc<FileInternal>> {
    if slib_dir.as_ref().is_some_and(|slib_dir| path.starts_with(slib_dir)) {
        let cell = {
            let mut cache = cache.lock().await;
            cache.entry(path.into()).or_insert_with(|| Arc::new(OnceCell::new())).clone()
        };
        let item = cell.get_or_init(|| async { load_file_inner(kind, path, mode).await }).await;

        item.as_ref().map(|v| v.clone())
    } else {
        return load_file_inner(kind, path, mode).await;
    }
}

async fn load_file_inner(kind: LoadFileKind, path: &Path, mode: LoadMode) -> Option<Arc<FileInternal>> {
    let content = read_file_cp850(path).await.ok()?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let hash = hasher.finish();
    match kind {
        LoadFileKind::Task => parse_task_file(&content, mode, hash),
        LoadFileKind::DPac => parse_dpac_file(&content, mode, hash),
        LoadFileKind::Text => parse_text_file(&content, mode, hash),
    }
    .ok()
    .map(Arc::new)
}
