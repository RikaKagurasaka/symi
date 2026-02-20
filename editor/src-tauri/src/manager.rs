use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock},
};

use parking_lot::RwLock;
use symi::{parse_source, AudioHandle, Compiler, Parse};

use crate::byte_char_mapper::ByteCharMapper;
pub type FileId = String;

pub struct LanguageManager {
    pub source: Arc<str>,
    pub parse: Parse,
    pub compiler: Compiler,
    pub byte_char_mapper: ByteCharMapper,
}

impl LanguageManager {
    pub fn new(source: Arc<str>) -> Self {
        let parse = parse_source(source.clone());
        let mut compiler = Compiler::new();
        let byte_char_mapper = ByteCharMapper::new(&source);
        compiler.compile(&parse.syntax_node());
        LanguageManager {
            source,
            parse,
            compiler,
            byte_char_mapper,
        }
    }
}

pub struct PolyManager {
    pub files: BTreeMap<FileId, LanguageManager>,
}

impl PolyManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(PolyManager {
            files: BTreeMap::new(),
        })
    }

    pub fn update_file(&mut self, file_id: FileId, source: String) {
        let lang_manager = LanguageManager::new(Arc::from(source));
        self.files.insert(file_id, lang_manager);
    }

    pub fn close_file(&mut self, file_id: &str) {
        self.files.remove(file_id);
    }
}
pub static MANAGER: LazyLock<Arc<RwLock<PolyManager>>> =
    LazyLock::new(|| Arc::new(RwLock::new(PolyManager::new().unwrap())));
pub static AUDIO_MANAGER: LazyLock<Arc<AudioHandle>> = LazyLock::new(|| {
    let handle = AudioHandle::new().expect("Failed to initialize AudioHandle");
    Arc::new(handle)
});
