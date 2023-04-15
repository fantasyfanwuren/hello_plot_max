use std::io::Write;

use super::userset::*;
use log::{error, info};
use prettytable::{Cell, Row, Table};
use tokio::time;

#[derive(Debug)]
pub struct DiskInfo {
    path: String,
    finished_num: usize,
    max_num: usize,
    remaining_size: f32,
    transfer_rate: f32,
    total_transfered: f32,
    transfer_state: bool,
}
#[derive(Debug)]
pub struct ShowInfos(Vec<DiskInfo>);

impl ShowInfos {
    pub async fn new(user_set: UserSet) -> Result<Self, Box<dyn std::error::Error>> {
        // 持续等待第一张图的出现
        println!("Waiting for the first plot file...");
        let plot_names = wait_polt(&user_set.source_dir_path).await?;
        let plot_name = plot_names[0].clone();
        info!("Get the first plot size{}", plot_name);

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
                remove_tmp(&item.path).await?;
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
                        item.size * 1000.0 * 1000.0 * 1000.0 * 1000.0 / 1024.0 / 1024.0 / 1024.0
                            + 2.0;
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
                    transfer_rate: 0.0,
                    total_transfered: 0.0,
                    transfer_state: false,
                });
            }
            ShowInfos(disks)
        };

        Ok(show_infos)
    }

    pub fn show(&self) {
        print!("{}[2J", 27 as char);
        print!("\x1b[H");
        std::io::stdout().flush().unwrap();
        // 基础信息展示
        println!(
            "This tool is open source and free of charge.My Chia donation address:xch1uhjvk0qm4sth2p3xlf0pv9x00w65ttfjh9h5eerynusnh7yuslqqhf2nm2"
        );

        // 作者信息展示：
        println!(
            "If you have any good ideas or find bug,please contact me with email:756423901@qq.com"
        );

        // 表格展示
        let mut table = Table::new();

        // 表头
        table.add_row(Row::new(vec![
            Cell::new("ID"),
            Cell::new("Final Path"),
            Cell::new("Max Allow Number"),
            Cell::new("Finished Number"),
            Cell::new("Remaining Size"),
            Cell::new("Current Rate"),
            Cell::new("Tatal Transfered"),
            Cell::new("Current State"),
        ]));

        // 内容
        for (id, item) in self.0.iter().enumerate() {
            let state: &str = {
                if item.finished_num >= item.max_num {
                    "Finished"
                } else {
                    if item.transfer_state {
                        "Transfering..."
                    } else {
                        "Waiting for transfer...."
                    }
                }
            };
            let transfer_rate = format!("{}M/s", item.transfer_rate);
            let total_transferde = format!("{}GB", item.total_transfered);
            table.add_row(Row::new(vec![
                Cell::new(&id.to_string()),
                Cell::new(&item.path),
                Cell::new(&item.max_num.to_string()),
                Cell::new(&item.finished_num.to_string()),
                Cell::new(&item.remaining_size.to_string()),
                Cell::new(&transfer_rate),
                Cell::new(&total_transferde),
                Cell::new(state),
            ]));
        }

        table.print_tty(true).unwrap();
    }

    pub async fn total_remaining(&self) -> usize {
        let mut result = 0_usize;
        for item in self.0.iter() {
            let remaining_num = item.max_num - item.finished_num;
            result += remaining_num;
        }
        result
    }

    pub async fn get_most_suitable_dir(
        &mut self,
        transfering_dirs: &Vec<String>,
        choose_plot_size: f32,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // 选择一个目录：它不应该正在传输中；然后优先选择有剩余空间的目录，若所有目录无法装下一张新图，则，则选择remaining_num最大的；

        // 获取最大remaining_siz和对应的目录路径
        let (max_remaining_size, final_path) = {
            let mut max_remaining_size = 0_f32;
            let mut final_path = "";
            for item in self.0.iter() {
                // 如果这个目录不被包括在传输线程中，且这个目录的剩余空间为当前最大
                if !transfering_dirs.contains(&item.path)
                    && item.remaining_size > max_remaining_size
                {
                    max_remaining_size = item.remaining_size;
                    final_path = &item.path;
                }
            }
            (max_remaining_size, final_path)
        };

        // 判断最大的剩余空间，是否大于被选择的文件大小，若是返回这个目录,不需要删除旧图腾出空间
        if max_remaining_size > choose_plot_size && !final_path.is_empty() {
            return Ok(Some(final_path.to_owned()));
        }

        // 若执行到这一步骤，意味着所有硬盘都是饱和状态了，则需要选择一个remaining_num最大的盘.并执行删图腾出空间的动作；
        let (max_remaining_num, id) = {
            let mut max_remaining_num = 0_usize;
            let mut id = 0_usize;
            for (ii, item) in self.0.iter().enumerate() {
                let remaining_num = item.max_num - item.finished_num;
                if !transfering_dirs.contains(&item.path) && remaining_num > max_remaining_num {
                    max_remaining_num = remaining_num;
                    id = ii;
                }
            }
            (max_remaining_num, id)
        };

        // 返回
        if max_remaining_num > 0 {
            // 执行删除动作，腾出空间，使得remaining_size大于 choose_plot_size
            self.del_plot(id, choose_plot_size).await?;
            Ok(Some(self.0[id].path.clone()))
        } else {
            Ok(None)
        }
    }

    async fn del_plot(
        &mut self,
        id: usize,
        choose_plot_size: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let final_path = self.0[id].path.clone();
        let plots = scan_plot(&final_path).await?;

        // 循环删除，直到remaining_size>choose_plot_size
        for plot in plots.iter() {
            if self.0[id].remaining_size > choose_plot_size {
                return Ok(());
            }
            // 获取删除路径
            let del_path = format!("{}/{}", final_path, plot);

            // 获取删除大小
            let del_size = get_plot_size(&del_path).await?;

            // 判断是否为新图，若为新图则跳过
            if del_size > choose_plot_size - 0.3 && del_size < choose_plot_size + 0.3 {
                continue;
            } else {
                // 执行删除
                std::fs::remove_file(&del_path)?;
                info!("[Thread main]:delete the old plot:{}", del_path);

                // 更新remaining_size
                self.0[id].remaining_size = self.0[id].remaining_size + del_size
            }
        }
        error!("[Thread main]:delete the old plot process fail.");
        Err("删除失败..".into())
    }

    pub fn change_state(&mut self, dir: &str) {
        for item in self.0.iter_mut() {
            if item.path == dir.to_string() {
                item.transfer_state = !item.transfer_state;
                item.transfer_rate = 0.0;
                item.total_transfered = 0.0;
                break;
            }
        }
    }

    pub fn updtate_transfering_msg(
        &mut self,
        dir: &str,
        transfer_rate: f32,
        total_transfered: f32,
    ) {
        for item in self.0.iter_mut() {
            if item.path == dir.to_owned() {
                item.transfer_rate = transfer_rate;
                item.total_transfered = total_transfered;
            }
        }
    }

    pub fn add_one_plot(&mut self, dir: &str, new_plot_size: f32) {
        for item in self.0.iter_mut() {
            if item.path == dir.to_owned() {
                item.remaining_size = item.remaining_size - new_plot_size;
                item.finished_num += 1;
            }
        }
    }
}

pub async fn wait_polt(source_dir_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let result = loop {
        let plot_names = scan_plot(source_dir_path).await?;
        if !plot_names.is_empty() {
            break plot_names;
        } else {
            time::sleep(time::Duration::from_secs(10)).await;
        }
    };
    Ok(result)
}

pub async fn get_plot_size(path: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len() as f32 / 1024.0 / 1024.0 / 1024.0;
    Ok(file_size)
}

pub async fn remove_tmp(final_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
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
