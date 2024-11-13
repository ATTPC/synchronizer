 # synchronizer

 > We impose order on the chaos of organic evolution. You exist because we allow it. And you will end because we demand it.
 >
 > -- Sovereign, Mass Effect

 AT-TPC data is messy. Running analysis on messy data is hard.

 The synchronizer is an effort to impose order on the chaos of runs in real data. It takes a set of AT-TPC runs and synchronize the data bits according to their time stamps.

 ## Installation

 The synchronizer is written in Rust and requires a Rust compiler. The Rust toolchain can be installed from [here](https://rust-lang.org).

 Once the Rust toolchain is installed, download the repository from GitHub

 ```bash
 git clone https://github.com/ATTPC/synchronizer.git
 ```

 From within the repository run

 ```bash
 cargo install
 ```

 This will install the synchronizer executable to your cargo installs and it will be available on your path as `synchronizer`.

 ## Use

 The synchronizer uses the following CLI:

 ```txt
 synchronizer --config/-c /path/to/some/config.yml
 ```

 where `/path/to/some/config.yml` should be replaced with the path to an appropriate configuration file. The synchronizer can generate a default template for you using the `new` command.

 ```txt
 synchronizer --config/-c /path/to/some/config.yml new
 ```

 ### Configuration

 Configurations are defined as the following YAML:

 ```yaml
 merger_path: "/path/to/some/merger/data/"
 sync_path: "/path/to/some/synchronic/data/"
 min_run: 55
 max_run: 69
 ```

 Some important notes:

 - The path given as the `sync_path` must exist before running the synchronizer
 - Min run and max run are the range of run numbers (*merger run numbers*) to be synchronized. The range is inclusive; run numbers can be missing in the range.

 ### Output Format

 The synchronizer follows the current [attpc_merger](https://github.com/attpc_merger) format. That format is

 ```txt
 run_0001.h5
 |---- events - min_event, max_event, version
 |    |---- event_#
 |    |    |---- get_traces(dset) - id, timestamp, timestamp_other
 |    |    |---- frib_physics - id, timestamp
 |    |    |    |---- 977(dset)
 |    |    |    |---- 1903(dset)
 ```

 ## Why would you do this to me?

 Because due to some gremlins roaming around the hardware during the experiment, the GET and FRIB DAQs didn't have the same busy and/or trigger!
 Or something was very wrong with the VMUSB!
