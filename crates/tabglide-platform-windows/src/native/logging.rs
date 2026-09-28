use super::Result;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

struct LogFile {
    path: PathBuf,
    file: Option<File>,
    bytes: u64,
}
impl LogFile {
    fn open(&mut self) -> io::Result<()> {
        fs::create_dir_all(self.path.parent().expect("log parent"))?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.bytes = file.metadata()?.len();
        self.file = Some(file);
        Ok(())
    }
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.bytes + bytes.len() as u64 > MAX_LOG_BYTES {
            self.file.take();
            let previous = self.path.with_extension("previous.log");
            match fs::remove_file(&previous) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            fs::rename(&self.path, previous)?;
            self.open()?;
        }
        if let Some(file) = &mut self.file {
            file.write_all(bytes)?;
            self.bytes += bytes.len() as u64;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct LogWriter(Arc<Mutex<LogFile>>);
impl Write for LogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("log lock poisoned"))?
            .write(buffer)?;
        Ok(buffer.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

pub(super) struct Diagnostics {
    enabled: Arc<AtomicBool>,
    writer: LogWriter,
}
impl Diagnostics {
    pub fn new(directory: PathBuf, enabled: bool) -> Result<Self> {
        let diagnostics = Self {
            enabled: Arc::new(AtomicBool::new(false)),
            writer: LogWriter(Arc::new(Mutex::new(LogFile {
                path: directory.join("TabGlide.log"),
                file: None,
                bytes: 0,
            }))),
        };
        diagnostics.set_enabled(enabled)?;
        let flag = diagnostics.enabled.clone();
        let layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(diagnostics.writer.clone())
            .with_filter(tracing_subscriber::filter::dynamic_filter_fn(
                move |_, _| flag.load(Ordering::Relaxed),
            ));
        tracing_subscriber::registry().with(layer).try_init()?;
        Ok(diagnostics)
    }
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        let mut log = self.writer.0.lock().map_err(|_| "log lock poisoned")?;
        if enabled && log.file.is_none() {
            log.open()?;
        }
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            log.file.take();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same_warning_callsite() {
        tracing::warn!("reloadable diagnostics regression");
    }

    #[test]
    fn reload_changes_logging_at_previously_visited_callsites() {
        let directory =
            std::env::temp_dir().join(format!("tabglide-logging-test-{}", std::process::id()));
        let path = directory.join("TabGlide.log");
        let diagnostics = Diagnostics::new(directory.clone(), false).unwrap();
        same_warning_callsite();
        assert!(!path.exists());
        diagnostics.set_enabled(true).unwrap();
        same_warning_callsite();
        diagnostics.set_enabled(false).unwrap();
        same_warning_callsite();
        diagnostics.set_enabled(true).unwrap();
        same_warning_callsite();
        diagnostics.set_enabled(false).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("reloadable diagnostics regression").count(), 2);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
