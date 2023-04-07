use std::io::Write;

use super::userset::*;
use log::info;
use prettytable::{Cell, Row, Table};
use tokio::time;

#[derive(Debug)]
pub struct DiskInfo {
    path: String,
    finished_num: usize,
    max_num: usize,
    remaining_size: f32,
    transfer_state: bool,
}
#[derive(Debug)]
pub struct ShowInfos(Vec<DiskInfo>);

impl ShowInfos {
    pub async fn new(user_set: UserSet) -> Result<Self, Box<dyn std::error::Error>> {
        // 持续等待第一张图的出现
        let plot_name = loop {
            println!("Waiting for the first plot file...");
            let plot_names = scan_plot(&user_set.source_dir_path).await?;
            if !plot_names.is_empty() {
                break plot_names[0].clone();
            } else {
                time::sleep(time::Duration::from_secs(10)).await;
            }
        };
        info!("等来了第一张图：{}", plot_name);

        // 计算第一张new plot文件的大小
        let new_plot_size = {
            let file_path = format!("{}/{}", user_set.source_dir_path, plot_name);
            get_plot_size(&file_path).await?
        };
        info!("得到了第一张图的大小为{}", new_plot_size);

        // 扫描每个盘的plot文件，计算出plot文件总空间，计算出新图总空间，根据已有这张图的大小和盘的allow_new_plots_num，预估出剩余容量
        let show_infos: ShowInfos = {
            let mut disks = vec![];
            for item in user_set.final_dirs {
                // 移除残留的temp文件
                remove_temp(&item.path).await?;
                info!("{}:Non plot file deletion completed.", item.path);

                // 统计所有的plots文件
                let plots = scan_plot(&item.path).await?;

                // 统计新图的数量和已完成plot文件总空间
                let (finished_num, finish_size) = {
                    let mut finished_num = 0_usize;
                    let mut finish_size = 0_f32;
                    for plot in plots {
                        let plot_path = format!("{}/{}", item.path, plot);
                        let plot_size = get_plot_size(&plot_path).await?;
                        finish_size += plot_size;
                        if plot_size > new_plot_size - 0.3 && plot_size < new_plot_size + 0.3 {
                            finished_num += 1;
                        }
                    }
                    (finished_num, finish_size)
                };
                info!(
                    "{}:There are {} new plots.All plot files occupy {}Gb of space.",
                    item.path, finished_num, finish_size
                );

                // 计算当前剩余空间
                let remaining_size =
                    item.size * 1000.0 * 1000.0 * 1000.0 * 1000.0 / 1024.0 / 1024.0 / 1024.0
                        - finish_size;
                info!(
                    "{}:Estimated idle space is {}Gb.",
                    item.path, remaining_size
                );

                // 计算最大p盘数量
                let max_num = {
                    let total_space =
                        item.size * 1000.0 * 1000.0 * 1000.0 * 1000.0 / 1024.0 / 1024.0 / 1024.0;
                    let max_num = total_space / new_plot_size;
                    max_num as usize
                };
                info!(
                    "{}:Maximum allowed number of new images is {}",
                    item.path, max_num
                );

                // 添加到disks
                disks.push(DiskInfo {
                    path: item.path,
                    finished_num,
                    max_num,
                    remaining_size,
                    transfer_state: false,
                });
            }
            ShowInfos(disks)
        };

        Ok(show_infos)
    }

    pub async fn show(&self) {
        print!("{}[2J", 27 as char);
        print!("\x1b[H");
        std::io::stdout().flush().unwrap();
        // 基础信息展示
        println!(
            "This tool is open source and free of charge.My Chia donation address:xch1uhjvk0qm4sth2p3xlf0pv9x00w65ttfjh9h5eerynusnh7yuslqqhf2nm2"
        );

        // 作者信息展示：
        println!("");

        // 表格展示
        let mut table = Table::new();

        // 表头
        table.add_row(Row::new(vec![
            Cell::new("ID"),
            Cell::new("Final Path"),
            Cell::new("Max Allow Number"),
            Cell::new("Finished Number"),
            Cell::new("Remaining Size"),
            Cell::new("Current State"),
        ]));

        // 内容
        for (id, item) in self.0.iter().enumerate() {
            let state: &str = {
                if item.finished_num == item.max_num {
                    "Finished"
                } else {
                    if item.transfer_state {
                        "Transfering..."
                    } else {
                        "Waiting for transfer...."
                    }
                }
            };
            table.add_row(Row::new(vec![
                Cell::new(&id.to_string()),
                Cell::new(&item.path),
                Cell::new(&item.max_num.to_string()),
                Cell::new(&item.finished_num.to_string()),
                Cell::new(&item.remaining_size.to_string()),
                Cell::new(state),
            ]));
        }

        table.print_tty(true).unwrap();
    }
}

pub async fn get_plot_size(path: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len() as f32 / 1024.0 / 1024.0 / 1024.0;
    Ok(file_size)
}

pub async fn remove_temp(final_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let files = std::fs::read_dir(final_dir)?;
    for file in files {
        let file_path = file?.path();
        if let Some(extension) = file_path.extension() {
            if extension != "plot" {
                std::fs::remove_file(file_path)?
            }
        }
    }
    Ok(())
}

pub async fn scan_plot(source_dir: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut result = vec![];
    let files = std::fs::read_dir(source_dir)?;
    for file in files {
        let file_path = file?.path();
        if let Some(extension) = file_path.extension() {
            if extension == "plot" {
                let file_path = file_path.file_name().unwrap().to_str().unwrap().to_owned();
                result.push(file_path);
            }
        }
    }
    Ok(result)
}
