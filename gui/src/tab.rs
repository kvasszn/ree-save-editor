use crate::{code_editor::CodeEditor, file::FileView};

pub struct Tab {
    pub tab: TabType,
    pub idx: u64,
}

pub enum TabType {
    SaveFileView(FileView),
    Script(CodeEditor)
}

impl Tab {
    pub fn load_script(path: &str, id: u64) -> Self {
        Self {
            tab: TabType::Script(CodeEditor::new(path, id)),
            idx: id
        }
    }
}

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
