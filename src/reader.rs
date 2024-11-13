//! Implementation of an attpc_merger Reader.
//! Also contains code to check and synchronize time stamps
use color_eyre::eyre::{eyre, Result};
use hdf5_metno::File;
use ndarray::{Array1, Array2};
use std::path::{Path, PathBuf};
// use hdf5_metno::types::VarLenUnicode;

/// Enum for what version of the merger we are dealing with.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum MergerVersion {
    V010,
    V020,
    Invalid,
}

/// Construct the formated run path from a parent path and run number.
pub fn construct_run_path(path: &Path, run_number: i32) -> PathBuf {
    path.join(format!("run_{:0>4}.h5", run_number))
}

/// Unified definition of a GET event from the merger
#[derive(Debug)]
pub struct GetEvent {
    pub traces: Array2<i16>,
    pub id: u32,
    pub timestamp: u64,
    pub timestamp_other: u64,
}

/// Unified definition of an FRIBDAQ event from the merger
#[derive(Debug)]
pub struct FribEvent {
    pub traces: Array2<u16>,
    pub coincidence: Array1<u16>,
    pub event: u32,
    pub timestamp: u32,
}

/// Unified definition of a complete event from the merger
#[derive(Debug)]
pub struct MergerEvent {
    pub get: Option<GetEvent>,
    pub frib: Option<FribEvent>,
    pub run_number: i32,
    pub event: u64,
}

/// Representation of a Reader for data from attpc_merger. It is
/// capable of determining which version of the merger produced the
/// data and then parsing it appropriately.
#[derive(Debug)]
pub struct MergerReader {
    merger_path: PathBuf,
    version: MergerVersion,
    current_run: i32,
    pub current_file: File,
    pub current_event: u64,
    current_max_event: u64,
    ts_get_sync: Vec<u64>,
    ts_frib_sync: Vec<u64>,
    pub get_sync: Vec<usize>,
    pub frib_sync: Vec<usize>,
}

impl MergerReader {
    /// Create a new reader. The run is opened and initialized.
    pub fn new(merger_path: &Path, run: i32) -> Result<Self> {
        let file = File::open(construct_run_path(merger_path, run))?;
        let mut reader = Self {
            merger_path: merger_path.to_path_buf(),
            version: MergerVersion::Invalid,
            current_run: run,
            current_file: file,
            current_event: 0,
            current_max_event: 0,
            ts_get_sync: Vec::<u64>::new(),
            ts_frib_sync: Vec::<u64>::new(),
            get_sync: Vec::<usize>::new(),
            frib_sync: Vec::<usize>::new(),
        };
        reader.init_file()?;
        Ok(reader)
    }

    // Read through all events and store all TS 
    pub fn read_ts(&mut self)  -> Result<()> {
        while self.current_event < self.current_max_event {
            let get_event = self.read_event()?;
            let frib_event = self.read_event()?;
            self.ts_get_sync.push(get_event.unwrap().get.unwrap().timestamp_other);
            self.ts_frib_sync.push(frib_event.unwrap().frib.unwrap().timestamp as u64);
            self.current_event += 1;
        }
        Ok(())
    }

    // Find synchronous TS between the GET and FRIB stream and make lists
    pub fn sync_ts(&mut self) {
        let mut tsd_get: Vec<i64> = Vec::new();
        let mut tsd_frib: Vec<i64> = Vec::new();
        // Calculate TS differences
        tsd_get.push(0);
        for i in 1..self.ts_get_sync.len() {
            tsd_get.push(self.ts_get_sync[i] as i64 - self.ts_get_sync[i-1] as i64);
        }
        tsd_frib.push(0);
        for i in 1..self.ts_frib_sync.len() {
            tsd_frib.push(self.ts_frib_sync[i] as i64 - self.ts_frib_sync[i-1] as i64);
        }
        // Check whether first events are aligned
        let mut offset: usize = 0;
        let mut get_first: usize = 0;
        let mut frib_first: usize = 0;
        if (tsd_get[1] - tsd_frib[1]).abs() > 100 { // not aligned!
            println!("First events are not aligned!");
            // Align time stamps after finding matching pattern of TS
            let mut get_pattern: i64;
            let mut frib_pattern: i64;
            let depth: usize = 5;
            for start in 0..tsd_get.len()/2 { // Good starting event should be before half of data!
                while offset < tsd_get.len()/2 && offset < tsd_frib.len()/2 {
                    get_pattern = 0;
                    frib_pattern = 0;
                    for j in 1..depth { // look for matching pattern over depth TS differences
                        get_pattern += (tsd_get[start +j + offset] - tsd_frib[start + j]).abs();
                        frib_pattern += (tsd_frib[start + j + offset] - tsd_get[start + j]).abs();
                    }
                    if get_pattern < 5 { // GET ahead of FRIB
                        get_first = start + offset;
                        frib_first = start;
                        println!("Fist aligned event is GET {}, FRIB {}", get_first, frib_first);
                        break;
                    }
                    if frib_pattern < 5 { // FRIB ahead of GET
                        frib_first = start + offset;
                        get_first = start;
                        println!("Fist aligned event is GET {}, FRIB {}", get_first, frib_first);
                        break;
                    }
                    offset += 1;
                }
                if get_first > 0 || frib_first > 0 { // Found a good match, get out
                    break;
                }
                offset = 0; // Didn't find a good match, try next start
            }
        }
        // Now compare differences within jitter margin and make list of matching events
        // let mut get_sync: Vec<usize> = Vec::new();
        // let mut frib_sync: Vec<usize> = Vec::new();
       // spdlog::info!("GET TS: {}, FRIB TS: {}", tsd_get.len(), tsd_frib.len());
        offset = 0;
        // set first synchronized event from alignment
        self.get_sync.push(get_first);
        self.frib_sync.push(frib_first);
        let mut jitter: i64;
        for i in 1..tsd_get.len() - get_first {
            if i < tsd_frib.len() - offset - frib_first {
                jitter = tsd_get[i+get_first] - tsd_frib[i+frib_first+offset];
                if jitter > 1000 { // FRIB stream got its next event earlier than GET
                    // spdlog::info!("Found time stamp mismatch at event {}: get={} vs frib={}", i, tsd_get[i], tsd_frib[i]);
                    offset += 1;
                } else if jitter > 5 {
                    println!("Found abnormal TS jitter of {} in event {}", jitter, i + get_first);
                }
                self.get_sync.push(i + get_first);
                self.frib_sync.push(i + offset + frib_first);
                // spdlog::info!("Get event {} matches FRIB event {}", get_evt[i], frib_evt[i]);
            } else { // no more FRIB events to sync
                break;
            }
        }
        // get_sync and frib_sync should contain lists of matching data
        println!("First GET event synchronized is {}", self.get_sync[0]);
        println!("Last GET event synchronized is {}", self.get_sync[self.get_sync.len()-1]);
        println!("A total of {} time stamp mismatches were found", offset);
    }

    /// Read the next event from the run set.
    /// If the currently open run is finished, the next run that
    /// exists within the range is opened. If there is no more data
    /// to be read it returns a None.
    pub fn read_event(&mut self) -> Result<Option<MergerEvent>> {
        let result = match self.version {
            MergerVersion::V020 => self.read_event_020(),
            MergerVersion::V010 => self.read_event_010(),
            MergerVersion::Invalid => Err(eyre!("Attempting to read event from invalid reader!")),
        };
        // self.current_event += 1;

        result
    }

    /// Initialize the current file, and update our state
    fn init_file(&mut self) -> Result<()> {
        let parent_groups = self.current_file.member_names()?;
        if parent_groups.contains(&String::from("meta")) {
            self.version = MergerVersion::V010;
            let meta_group = self.current_file.group("meta")?;
            let meta_data = meta_group.dataset("meta")?;
            let meta_array = meta_data.read_1d::<u64>()?;
            self.current_event = meta_array[0];
            self.current_max_event = meta_array[2];
        } else if parent_groups.contains(&String::from("events")) {
            self.version = MergerVersion::V020;
            let event_group = self.current_file.group("events")?;
            self.current_event = event_group.attr("min_event")?.read_scalar::<u64>()?;
            self.current_max_event = event_group.attr("max_event")?.read_scalar::<u64>()?;
        } else {
            return Err(eyre!("Invalid Merger Version!"));
        }

        Ok(())
    }

    /// Find the next available file in the run range.
    /// If there are no more runs, returns None.
    // fn find_next_file(&mut self) -> Result<Option<()>> {
    //     let mut path;
    //     loop {
    //         self.current_run += 1;
    //         if self.current_run > self.max_run {
    //             return Ok(None);
    //         }
    //         path = construct_run_path(&self.merger_path, self.current_run);
    //         if !path.exists() {
    //             continue;
    //         }
    //         break;
    //     }
    //     self.current_file = File::open(path)?;
    //     self.init_file()?;
    //     Ok(Some(()))
    // }

    /// Read an event from the modern merger format.
    fn read_event_020(&mut self) -> Result<Option<MergerEvent>> {
        let event_group = self
            .current_file
            .group("events")?
            .group(&format!("event_{}", self.current_event))?;

        let mut maybe_get = None;
        let mut maybe_frib = None;
        if let Ok(get_data) = event_group.dataset("get_traces") {
            maybe_get = Some(GetEvent {
                traces: get_data.read_2d()?,
                id: get_data.attr("id")?.read_scalar()?,
                timestamp: get_data.attr("timestamp")?.read_scalar()?,
                timestamp_other: get_data.attr("timestamp_other")?.read_scalar()?,
            });
        }
        if let Ok(frib_group) = event_group.group("frib_physics") {
            let frib_977 = frib_group.dataset("977")?;
            let frib_1903 = frib_group.dataset("1903")?;
            maybe_frib = Some(FribEvent {
                traces: frib_1903.read_2d()?,
                coincidence: frib_977.read_1d()?,
                event: frib_group.attr("event")?.read_scalar()?,
                timestamp: frib_group.attr("timestamp")?.read_scalar()?,
            })
        }
        Ok(Some(MergerEvent {
            get: maybe_get,
            frib: maybe_frib,
            run_number: self.current_run,
            event: self.current_event,
        }))
    }

    /// Read an event from the 0.1.0 merger format
    fn read_event_010(&mut self) -> Result<Option<MergerEvent>> {
        let mut maybe_get = None;
        let mut maybe_frib = None;
        let get_group = self.current_file.group("get")?;
        if let Ok(get_data) = get_group.dataset(&format!("evt{}_data", self.current_event)) {
            let get_header = get_group
                .dataset(&format!("evt{}_header", self.current_event))?
                .read_1d::<f64>()?;
            maybe_get = Some(GetEvent {
                traces: get_data.read_2d()?,
                id: get_header[0] as u32,
                timestamp: get_header[1] as u64,
                timestamp_other: get_header[2] as u64,
            });
        }
        let frib_evt_group = self.current_file.group("frib")?.group("evt")?;
        if let Ok(frib_1903_data) =
            frib_evt_group.dataset(&format!("evt{}_1903", self.current_event))
        {
            let frib_977_data =
                frib_evt_group.dataset(&format!("evt{}_977", self.current_event))?;
            let frib_header = frib_evt_group
                .dataset(&format!("evt{}_header", self.current_event))?
                .read_1d::<u32>()?;
            maybe_frib = Some(FribEvent {
                traces: frib_1903_data.read_2d()?,
                coincidence: frib_977_data.read_1d()?,
                event: frib_header[0],
                timestamp: frib_header[1],
            });
        }
        Ok(Some(MergerEvent {
            get: maybe_get,
            frib: maybe_frib,
            run_number: self.current_run,
            event: self.current_event,
        }))
    }
}
