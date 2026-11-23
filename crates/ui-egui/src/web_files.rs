//! Browser file requests and byte buffers. The host owns the picker and commits Files writes.

use std::path::{Path, PathBuf};

use projectcraft_engine::{CmdError, Session, interchange::Format};

use crate::{App, FileRequest, Selection, calendar_notice, import_report, is_import, resource_sheet};

impl App {
    /// Hand one browser file action to the host after an egui frame.
    pub fn take_web_request(&mut self) -> Option<&'static str> {
        if std::mem::take(&mut self.web_save) {
            return Some("save");
        }
        Some(match self.take_file_request()? {
            FileRequest::Open => "open",
            FileRequest::SaveAs => "save-as",
            FileRequest::Export(Format::Mspdi) => "export-mspdi",
            FileRequest::Export(Format::Csv) => "export-csv",
        })
    }

    /// Include in-progress cell edits before a host close or file replacement.
    pub fn web_dirty(&self) -> bool {
        self.is_dirty() || self.editing.is_some() || self.resources.editing.is_some()
    }

    fn finish_web_edits(&mut self) {
        if let Some((id, field, text)) = self.editing.take() {
            self.commit(id, field, text);
        }
        if let Some((id, field, text)) = self.resources.editing.take() {
            self.commit_resource(id, field, text);
        }
    }

    /// Opens or imports a host-provided file. Invalid input leaves the current project intact.
    pub fn open_web(&mut self, name: &str, bytes: &[u8]) -> Result<(), CmdError> {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(CmdError::Io("Project files must be 128 MiB or smaller".into()));
        }
        let path = Path::new(name);
        let imported = is_import(path);
        let session = if imported {
            let format = if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("xml")) { Format::Mspdi } else { Format::Csv };
            let mut session = Session::import(format, bytes)?;
            if format == Format::Csv
                && let Some(stem) = path.file_stem().and_then(|part| part.to_str())
            {
                session.project.name = stem.to_owned();
            }
            session
        } else {
            let text = std::str::from_utf8(bytes).map_err(|e| CmdError::Io(e.to_string()))?;
            Session::load_json(text)?
        };
        let report = session.import_report().cloned();
        self.saved = session.project.clone();
        self.session = session;
        self.path = (!imported).then(|| PathBuf::from(name));
        self.editing = None;
        self.sel = Selection::default();
        self.resources = resource_sheet::ResourceSheet::default();
        self.unsaved_import = imported;
        self.import_report = report.map(|r| (import_report::file_name(path), r));
        self.status = calendar_notice(&self.session.project).unwrap_or_default();
        Ok(())
    }

    /// Serializes the server-independent ProjectCraft project.
    pub fn save_web(&mut self) -> Result<String, CmdError> {
        self.finish_web_edits();
        self.session.save_json()
    }

    /// Marks the exact confirmed bytes clean, preserving edits made after the save started.
    pub fn saved_web(&mut self, name: &str, snapshot: &str) -> Result<(), CmdError> {
        self.saved = Session::load_json(snapshot)?.project;
        self.path = Some(PathBuf::from(name));
        self.unsaved_import = false;
        Ok(())
    }

    /// Exports another program's format from the same computed schedule.
    pub fn export_web(&mut self, format: Format) -> Result<String, CmdError> {
        self.finish_web_edits();
        self.session.export(format)
    }
}
