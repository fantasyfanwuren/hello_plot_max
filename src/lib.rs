pub mod show;
pub mod userset;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task;

use log::{debug, info, warn};

pub use show::*;
use tokio::time;
pub use userset::*;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 获取用户设置
    let user_set = get_user_set().await?;
    info!("[Thread main]:Get user set:{:?}.", user_set);

    // 获取限制速度
    let hdd_limit_rate = user_set.hdd_limit_rate;
    debug!("[Thread main]:Draw hdd_limit_rate:{}.", hdd_limit_rate);

    // 生成分布图及获取 source_dir_path
    let source_dir_path = user_set.source_dir_path.clone();
    debug!("[Thread main]:Draw source_dir_path:{}", source_dir_path);
    let s = ShowInfos::new(user_set).await?;
    s.show();

    // 筹备Mutex:给用户展示的信息，正在进行转移的线程标记，线程数组：handles
    let show_infos = Arc::new(Mutex::new(s));
    let transfering_plots: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let transfering_dirs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let mut handles = vec![];
    debug!("[Thread main]:show_infos,transfering_plots,transfering_dirs Arc and Mutex created successfully.");

    // 循环判断，直到所有的finished_num == max_num
    'wait_plots: loop {
        // 判定整体剩余，若为0则推出
        let total_remaining = {
            let show_infos_lock = show_infos.lock().unwrap();
            let result = show_infos_lock.total_remaining().await;
            drop(show_infos_lock);
            info!("[Thread main]:The remaining number of plots is{}", result);
            result
        };
        if total_remaining == 0 {
            info!("[Thread main]:The remaining number of plots is 0,plot task finished");
            break 'wait_plots;
        }
        // 等待源目录出现plot文件
        let plot_names = wait_polt(&source_dir_path).await?;
        info!(
            "[Thread main]:Scan the source path {},get: {:?}",
            source_dir_path, plot_names
        );

        // 选择一个plot文件:它不应该正在传输中
        let choose_plot = {
            let mut result = String::new();
            let mut transfering_plots_lock = transfering_plots.lock().unwrap();
            for plot in plot_names.iter() {
                if !transfering_plots_lock.contains(plot) {
                    result = plot.clone();
                    break;
                }
            }
            transfering_plots_lock.push(result.clone());
            drop(transfering_plots_lock);
            info!(
                "[Thread main]:Select a plot that has not been transferred:{}",
                result
            );
            debug!(
                "[Thread main]:Push {} into transfering_plots variable",
                result
            );
            result
        };

        // 计算被选择的新plot 文件的大小
        let choose_plot_path = format!("{}/{}", source_dir_path, choose_plot);
        let choose_plot_size = get_plot_size(&choose_plot_path).await?;
        info!(
            "[Thread main]:Calculate the size of the selected plot file as {}GB",
            choose_plot_size
        );

        // 选择一个目录：它不应该正在传输中；然后优先选择有剩余空间的目录，若所有目录都满了，则选择remaining_num最大的；
        let choose_final_path = {
            let transfering_dirs_lock = transfering_dirs.lock().unwrap();
            let mut show_infos_lock = show_infos.lock().unwrap();
            let result = show_infos_lock
                .get_most_suitable_dir(&transfering_dirs_lock, choose_plot_size)
                .await?;
            drop(transfering_dirs_lock);
            drop(show_infos_lock);
            result
        };

        // 如果能选出就开启线程，如果不能选出，就等待10秒
        match choose_final_path {
            None => {
                warn!("[Thread main]:Can`t find the most suitable directory");
                time::sleep(time::Duration::from_secs(10)).await;
            }
            Some(final_path) => {
                info!(
                    "[Thread main]:Find the most suitable directory:{:?}",
                    final_path
                );
                debug!(
                    "[Thread main]:New thread will be opened:thread[{}]",
                    final_path
                );
                // 开启一个线程
                let transfering_plots = Arc::clone(&transfering_plots);
                let transfering_dirs = Arc::clone(&transfering_dirs);
                let show_infos = Arc::clone(&show_infos);
                debug!("[Thread main]:Copying Arc of transfering_plots,transfering_dirs and show_infos is accomplish");
                let handle = task::spawn(async move {
                    // 更新show_infs和transfering_plots
                    {
                        let mut transfering_dirs_lock = transfering_dirs.lock().unwrap();
                        let mut show_infos_lock = show_infos.lock().unwrap();
                        transfering_dirs_lock.push(final_path.clone());
                        show_infos_lock.change_state(&final_path);
                        show_infos_lock.show();
                        drop(show_infos_lock);
                        drop(transfering_dirs_lock);
                        debug!(
                            "[Thread {}]:Change state to transfering and transfering_dirs updated",
                            final_path
                        )
                    }

                    // 移动文件 choose_plot_path final_path choose_plot
                    let source_path = Path::new(&choose_plot_path);
                    let temp_name = format!("{}.tmp", choose_plot);
                    let target_path = Path::new(&final_path).join(&temp_name);
                    debug!(
                        "[Thread {}]:Get the sourse path:{:?}",
                        final_path, source_path
                    );
                    debug!(
                        "[Thread {}]:Get the target path:{:?}",
                        final_path, target_path
                    );

                    let mut source_file = std::fs::File::open(source_path).unwrap();
                    let mut target_file = std::fs::File::create(target_path).unwrap();

                    let mut buffer = [0; 1024 * 200];
                    let mut total_bytes = 0;

                    let start_time = time::Instant::now();

                    let mut read_time = 0;
                    loop {
                        let bytes_read = source_file.read(&mut buffer).unwrap();

                        if bytes_read == 0 {
                            break;
                        }

                        target_file.write_all(&buffer[..bytes_read]).unwrap();
                        // time::sleep(wait_time).await;

                        total_bytes += bytes_read;
                        let total_bytes_gb = total_bytes as f32 / 1024.0 / 1024.0 / 1024.0;

                        let elapsed_time = start_time.elapsed().as_secs_f32();
                        let transfer_rate = total_bytes as f32 / elapsed_time / 1024.0 / 1024.0;

                        if read_time % 4000 == 0 {
                            {
                                let mut show_infos_lock = show_infos.lock().unwrap();
                                show_infos_lock.updtate_transfering_msg(
                                    &final_path,
                                    transfer_rate,
                                    total_bytes_gb,
                                );
                                show_infos_lock.show();
                                drop(show_infos_lock);
                            }
                        }

                        if transfer_rate > hdd_limit_rate {
                            let sleep_time = time::Duration::from_millis(
                                (bytes_read as f32 / 100.0 / 1024.0 * 1000.0) as u64,
                            );
                            time::sleep(sleep_time).await;
                        }
                        read_time += 1;
                    }

                    // 删除源文件
                    std::fs::remove_file(&choose_plot_path).unwrap();
                    info!(
                        "[Thread {}]:{}:Successfully deleted",
                        final_path, choose_plot_path
                    );
                    // 修改文件名
                    let temp_path = format!("{}/{}", final_path, temp_name);
                    let target_path = format!("{}/{}", final_path, choose_plot);
                    info!(
                        "[Thread {}]:将{}重命名为{}",
                        final_path, temp_path, target_path
                    );
                    std::fs::rename(temp_path, target_path).unwrap();

                    // 更新多线程三项数据
                    {
                        let mut transfering_plots_lock = transfering_plots.lock().unwrap();
                        let mut transfering_dirs_lock = transfering_dirs.lock().unwrap();
                        let mut show_infos_lock = show_infos.lock().unwrap();

                        // 更新正在传输的plot文件
                        transfering_plots_lock.retain(|x| x != &choose_plot);

                        // 更新正在传输的最终目录
                        transfering_dirs_lock.retain(|x| x != &final_path);

                        // 更新show_info
                        show_infos_lock.change_state(&final_path);
                        show_infos_lock.update_remaining_size(&final_path, choose_plot_size);
                        show_infos_lock.show();
                        info!("[Thread {}],transfering_plots_lock,transfering_dirs_lock,show_infos_lock updated ,the thread out.", final_path);
                        drop(transfering_plots_lock);
                        drop(transfering_dirs_lock);
                        drop(show_infos_lock);
                    }
                });
                handles.push(handle);
            }
        }
    }

    for handle in handles {
        handle.await.unwrap();
    }

    Ok(())
}
