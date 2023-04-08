### Main Features:

- The software can intelligently identify old plots based on file size and delete them while drawing new plots. 
- It also automatically assigns threads for synchronous transmission.
- It prioritizes assigning space to hard disks with more remaining space, and then to hard disks with fewer new plots.

### How to Get

Please click "release" on the right side of this page, download the latest compressed package, and then extract it to any folder on your local machine.

### Perform User Settings

Open the extracted folder.Open the userset.json file:
```json
{
    "source_dir_path": "/mnt/temp/final",
    "hdd_limit_rate": 120.0,
    "final_dirs": [
        {
            "path": "/mnt/okchia/072/new",
            "size": 16.0
        },
        {
            "path": "/mnt/okchia/071/new",
            "size": 16.0
        },
        {
            "path": "/mnt/okchia/070/new",
            "size": 16.0
        }
    ]
}
```
* source_dir_path:
After the plotter finishes drawing, the location of the plot file.

* hdd_limit_rate:
The maximum transfer speed for each hard drive during the distribution process.
* path:
The final directory to which you want to distribute the plot file.

* size:
What is the capacity of this hard drive? You can fill in the capacity according to the manufacturer's specifications, in units of terabytes (T). Usually, you can enter 16.0, 14.0, 12.0, 10.0, etc. here.

### Run
* Grant permission to this tool.
```
$ sudo chmod +x hello_plot_max
```
* run
```
$ sudo ./hello_plot_max
```
### View And Set The Log
* view the log 
```
$ cd log
$ tail -f requests.log
```

* set the log
Open the log4rs.yaml file:

```yaml
refresh_rate: 30 seconds

appenders:
  console:
    kind: console

  requests:
    kind: file
    path: "./log/requests.log"
    encoder:
      pattern: "[{d(%Y-%m-%d %H:%M:%S)} {l}] - {m}{n}"

root:
  level: debug
  appenders:
    - requests
    - console
```
* level:
It includes four levels: debug, info, warn, error.The higher the level, the fewer logs are displayed.

* requests
Display logs in the file : requests.log

* console
Display logs in the console.

### End