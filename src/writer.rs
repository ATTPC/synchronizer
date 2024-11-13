//! Representation of a Writer for harmonic data
use super::reader::{construct_run_path, MergerEvent};
use color_eyre::eyre::Result;
use hdf5_metno::types::VarLenUnicode;
use hdf5_metno::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Representation of a writer for harmonic data.
/// It writes data with a slightly modified version of the
/// 0.2.0 merger format (see README). Harmonic data is written
/// to files, where each file has the same total amount of data
/// (in bytes).
#[derive(Debug)]
pub struct SyncWriter {
    sync_path: PathBuf,
    current_path: PathBuf,
    pub current_file: File,
    pub current_run: i32,
    current_event: u64,
}

impl SyncWriter {
    /// Create a new writer, the file to be written is initialized.
    pub fn new(sync_path: &Path, run: i32) -> Result<Self> {
        let current_run = run;
        let current_path = construct_run_path(sync_path, current_run);
        let current_file = File::create(&current_path)?;

        let writer = Self {
            sync_path: sync_path.to_path_buf(),
            current_path,
            current_file,
            current_run,
            current_event: 0,
        };

        writer.init_file()?;

        Ok(writer)
    }

    // Write synchronized run using event lists
    // pub fn write_run(&mut self, reader: MergerReader) {
    //     for i in 0..reader.get_sync.len() {
    //         // Read GET data
    //         reader.current_event = reader.get_sync[i];
    //         let get_result = match reader.version {
    //             MergerVersion::V020 => reader.read_event_020(),
    //             MergerVersion::V010 => reader.read_event_010(),
    //             MergerVersion::Invalid => Err(eyre!("Attempting to read event from invalid reader!")),
    //         };
    //         // Read FRIB data
    //         reader.current_event = reader.frib_sync[i];
    //         let frib_result = match reader.version {
    //             MergerVersion::V020 => reader.read_event_020(),
    //             MergerVersion::V010 => reader.read_event_010(),
    //             MergerVersion::Invalid => Err(eyre!("Attempting to read event from invalid reader!")),
    //         };
    //         // Write combined event
    //         self.write_combined(get_result, frib_result);
    //     }
    // }

    /// Write a MergerEvent combined from GET and FRIB pieces.
    pub fn write_combined(&mut self, get_event: MergerEvent, frib_event: MergerEvent) -> Result<()> {
        let event_group = self
            .current_file
            .group("events")?
            .create_group(&format!("event_{}", self.current_event))?;

        if let Some(get) = get_event.get.as_ref() {
            let traces = event_group
                .new_dataset_builder()
                .with_data(&get.traces)
                .create("get_traces")?;
            traces
                .new_attr::<u32>()
                .create("id")?
                .write_scalar(&get.id)?;
            traces
                .new_attr::<u64>()
                .create("timestamp")?
                .write_scalar(&get.timestamp)?;
            traces
                .new_attr::<u64>()
                .create("timestamp_other")?
                .write_scalar(&get.timestamp_other)?;
        }

        if let Some(frib) = frib_event.frib.as_ref() {
            let frib_group = event_group.create_group("frib_physics")?;
            frib_group
                .new_attr::<u32>()
                .create("event")?
                .write_scalar(&frib.event)?;
            frib_group
                .new_attr::<u32>()
                .create("timestamp")?
                .write_scalar(&frib.timestamp)?;
            frib_group
                .new_dataset_builder()
                .with_data(&frib.traces)
                .create("1903")?;
            frib_group
                .new_dataset_builder()
                .with_data(&frib.coincidence)
                .create("977")?;
        }

        self.current_event += 1;

        Ok(())
    }

    /// Write a MergerEvent.
    // pub fn write(&mut self, event: MergerEvent) -> Result<()> {
    //     let event_group = self
    //         .current_file
    //         .group("events")?
    //         .create_group(&format!("event_{}", self.current_event))?;

    //     event_group
    //         .new_attr::<i32>()
    //         .create("orig_run")?
    //         .write_scalar(&event.run_number)?;

    //     event_group
    //         .new_attr::<u64>()
    //         .create("orig_event")?
    //         .write_scalar(&event.event)?;

    //     if let Some(get) = event.get.as_ref() {
    //         let traces = event_group
    //             .new_dataset_builder()
    //             .with_data(&get.traces)
    //             .create("get_traces")?;
    //         traces
    //             .new_attr::<u32>()
    //             .create("id")?
    //             .write_scalar(&get.id)?;
    //         traces
    //             .new_attr::<u64>()
    //             .create("timestamp")?
    //             .write_scalar(&get.timestamp)?;
    //         traces
    //             .new_attr::<u64>()
    //             .create("timestamp_other")?
    //             .write_scalar(&get.timestamp_other)?;
    //     }

    //     if let Some(frib) = event.frib.as_ref() {
    //         let frib_group = event_group.create_group("frib_physics")?;
    //         frib_group
    //             .new_attr::<u32>()
    //             .create("event")?
    //             .write_scalar(&frib.event)?;
    //         frib_group
    //             .new_attr::<u32>()
    //             .create("timestamp")?
    //             .write_scalar(&frib.timestamp)?;
    //         frib_group
    //             .new_dataset_builder()
    //             .with_data(&frib.traces)
    //             .create("1903")?;
    //         frib_group
    //             .new_dataset_builder()
    //             .with_data(&frib.coincidence)
    //             .create("977")?;
    //     }

    //     self.current_event += 1;

    //     if self.current_path.metadata()?.len() >= self.harmonic_size {
    //         self.finish_file()?;
    //         self.current_event = 0;
    //         self.current_run += 1;
    //         self.current_path = construct_run_path(&self.harmonic_path, self.current_run);
    //         self.current_file = File::create(&self.current_path)?;
    //         self.init_file()?;
    //     }

    //     Ok(())
    // }

    /// Close the writer, ensuring that the required metadata
    /// is written to the current file.
    pub fn close(&self) -> Result<()> {
        self.finish_file()
    }

    /// Initialize the current file
    fn init_file(&self) -> Result<()> {
        let synchronizer_version =
            format!("{}:{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

        let events_group = self.current_file.create_group("events")?;
        events_group
            .new_attr::<u64>()
            .create("min_event")?
            .write_scalar(&0)?;
        events_group.new_attr::<u64>().create("max_event")?;
        events_group
            .new_attr::<VarLenUnicode>()
            .create("version")?
            .write_scalar(&VarLenUnicode::from_str(&synchronizer_version).unwrap())?;
        Ok(())
    }

    /// Write the required metadata to the currently open file
    /// when we are done with it.
    fn finish_file(&self) -> Result<()> {
        self.current_file
            .group("events")?
            .attr("max_event")?
            .write_scalar(&self.current_event)?;

        Ok(())
    }
}
