use crate::{file::FileView};

#[cfg(not(target_arch = "wasm32"))]
use crate::code_editor::CodeEditor;

pub struct Tab {
    pub tab: TabType,
    pub idx: u64,
}

pub enum TabType {
    SaveFileView(FileView),
    #[cfg(not(target_arch = "wasm32"))]
    Script(CodeEditor)
}

impl Tab {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_script(path: &str, id: u64) -> Self {
        Self {
            tab: TabType::Script(CodeEditor::new(path, id)),
            idx: id
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<CodeEditor> for Tab {
    fn from(value: CodeEditor) -> Self {
        let idx = value.idx;
        Self {
            tab: TabType::Script(value),
            idx
        }
    }
}

impl From<FileView> for Tab {
    fn from(value: FileView) -> Self {
        let idx = value.idx;
        Self {
            tab: TabType::SaveFileView(value),
            idx
        }
    }
}
