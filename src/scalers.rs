//! Functions for processing the scalers from a run.
// use std::path::Path;

// use super::reader::construct_run_path;
use color_eyre::eyre::{eyre, Result};
// use hdf5_metno::File;
// use polars::prelude::*;
use crate::MergerReader;
use crate::SyncWriter;
// use hdf5_metno::types::VarLenUnicode;
// use std::str::FromStr;


// Function to copy scalers from original HDF5 (version 010) to synchronized HDF5 (version 020)
pub fn copy_scalers_010(reader: &MergerReader, writer: &SyncWriter) -> Result<()> {
    let scaler_group = reader.current_file.group("frib")?.group("scaler")?;
    let scaler_groupw = writer.current_file.create_group("scalers")?;
    let mut scaler: u32 = 0;
    loop {
        if let Ok(event) = scaler_group.dataset(&format!("scaler{scaler}_data")) {
            let data = event.read_1d::<u32>()?;
            let scaler_eventw = scaler_groupw
                .new_dataset_builder()
                .with_data(&data)
                .create(format!("event_{}", scaler).as_str())?;
            let header = scaler_group.dataset(&format!("scaler{scaler}_header"));
            let attr = header?.read_1d::<u32>()?;
            scaler_eventw.new_attr::<u32>().create("start_offset")?.write_scalar(&attr[0])?;
            scaler_eventw.new_attr::<u32>().create("stop_offset")?.write_scalar(&attr[1])?;
            scaler_eventw.new_attr::<u32>().create("timestamp")?.write_scalar(&attr[2])?;
            scaler_eventw.new_attr::<u32>().create("incremental")?.write_scalar(&attr[4])?;
        } else {
            break;
        }
        scaler += 1;
    }
    scaler_groupw.new_attr::<u32>().create("min_event")?.write_scalar(&0)?;
    scaler_groupw.new_attr::<u32>().create("max_event")?.write_scalar(&scaler)?;
    Ok(())
}

// Function to copy scalers from original HDF5 (version 020) to synchronized HDF5 (version 020)
pub fn copy_scalers_020(reader: &MergerReader, writer: &SyncWriter) -> Result<()> {
    let scaler_groupw = writer.current_file.create_group("scalers")?;
    let scaler_group = reader.current_file.group("scalers")?;
    let scaler_min = scaler_group.attr("min_event")?.read_scalar::<u32>()?;
    let scaler_max = scaler_group.attr("max_event")?.read_scalar::<u32>()?;
    for scaler in scaler_min..(scaler_max + 1) {
        if let Ok(event) = scaler_group.dataset(&format!("event{scaler}_data")) {
            let data = event.read_1d::<u32>()?;
            let scaler_eventw = scaler_groupw
                .new_dataset_builder()
                .with_data(&data)
                .create(format!("event_{}", scaler).as_str())?;
            let start_offset = event.attr("start_offset")?.read_scalar::<u32>()?;
            let stop_offset = event.attr("stop_offset")?.read_scalar::<u32>()?;
            let timestamp = event.attr("timestamp")?.read_scalar::<u32>()?;
            let incremental = event.attr("incremental")?.read_scalar::<u32>()?;
            scaler_eventw.new_attr::<u32>().create("start_offset")?.write_scalar(&start_offset)?;
            scaler_eventw.new_attr::<u32>().create("stop_offset")?.write_scalar(&stop_offset)?;
            scaler_eventw.new_attr::<u32>().create("timestamp")?.write_scalar(&timestamp)?;
            scaler_eventw.new_attr::<u32>().create("incremental")?.write_scalar(&incremental)?;
        }
    }
    let min_event = scaler_group.attr("min_event")?.read_scalar::<u32>()?;
    let max_event = scaler_group.attr("max_event")?.read_scalar::<u32>()?;
    scaler_groupw.new_attr::<u32>().create("min_event")?.write_scalar(&min_event)?;
    scaler_groupw.new_attr::<u32>().create("max_event")?.write_scalar(&max_event)?;
    Ok(())
}