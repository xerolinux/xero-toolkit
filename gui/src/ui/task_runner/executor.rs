//! Command execution logic and running context management.
//!
//! This module handles the actual execution of commands, including:
//! - Process spawning and management
//! - Output capture (stdout/stderr)
//! - Error handling and result processing
//! - Command resolution (privilege escalation, AUR helpers)

use super::command::{Command, CommandResult, CommandType, TaskStatus};
use super::widgets::TaskRunnerWidgets;
use crate::core;
use crate::core::daemon::get_xero_auth_path;
use gtk4::glib;
use log::{error, info, warn};
use std::cell::RefCell;
use std::rc::Rc;
use xero_auth::utils::read_buffer_with_line_processing;

/// Context for a running command execution.
pub struct RunningContext {
    pub widgets: Rc<TaskRunnerWidgets>,
    pub commands: Rc<Vec<Command>>,
    pub index: usize,
    pub cancelled: Rc<RefCell<bool>>,
    /// PID of the currently running child process, if any - lets the
    /// Cancel button send it a real signal instead of just waiting.
    pub current_pid: Rc<RefCell<Option<u32>>>,
    exit_result: RefCell<Option<CommandResult>>,
}

impl RunningContext {
    /// Create a new running command context.
    pub fn new(
        widgets: Rc<TaskRunnerWidgets>,
        commands: Rc<Vec<Command>>,
        index: usize,
        cancelled: Rc<RefCell<bool>>,
        current_pid: Rc<RefCell<Option<u32>>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            widgets,
            commands,
            index,
            cancelled,
            current_pid,
            exit_result: RefCell::new(None),
        })
    }

    /// Set the exit result for the current command.
    pub fn set_exit_result(self: &Rc<Self>, result: CommandResult) {
        *self.exit_result.borrow_mut() = Some(result);
        self.try_finalize();
    }

    /// Try to finalize the current command.
    fn try_finalize(self: &Rc<Self>) {
        let result = {
            let mut exit_result = self.exit_result.borrow_mut();
            exit_result.take()
        };

        let Some(result) = result else {
            return;
        };

        self.current_pid.borrow_mut().take();

        if *self.cancelled.borrow() {
            self.widgets
                .update_task_status(self.index, TaskStatus::Cancelled);
            finalize_execution(&self.widgets, false, super::CANCELLED_MESSAGE);
            return;
        }

        match result {
            CommandResult::Success => {
                self.widgets.append_colored("\n[Exit code: 0]\n", "stdout");

                self.widgets
                    .update_task_status(self.index, TaskStatus::Success);
                execute_commands(
                    self.widgets.clone(),
                    self.commands.clone(),
                    self.index + 1,
                    self.cancelled.clone(),
                    self.current_pid.clone(),
                );
            }
            CommandResult::Failure { exit_code } => {
                let exit_msg = match exit_code {
                    Some(code) => format!("\n[Exit code: {}]\n", code),
                    None => "\n[Exit code: unknown]\n".to_string(),
                };
                self.widgets.append_colored(&exit_msg, "stderr");

                self.widgets
                    .update_task_status(self.index, TaskStatus::Failed);

                let exit_msg = exit_code
                    .map(|code| format!(" (exit code: {})", code))
                    .unwrap_or_default();
                let final_message = format!(
                    "Operation failed at step {} of {}{}",
                    self.index + 1,
                    self.commands.len(),
                    exit_msg
                );

                finalize_execution(&self.widgets, false, &final_message);
            }
        }
    }
}

/// Execute a sequence of commands.
pub fn execute_commands(
    widgets: Rc<TaskRunnerWidgets>,
    commands: Rc<Vec<Command>>,
    index: usize,
    cancelled: Rc<RefCell<bool>>,
    current_pid: Rc<RefCell<Option<u32>>>,
) {
    if *cancelled.borrow() {
        if index < commands.len() {
            widgets.update_task_status(index, TaskStatus::Cancelled);
        }
        finalize_execution(&widgets, false, super::CANCELLED_MESSAGE);
        return;
    }

    if index >= commands.len() {
        finalize_execution(&widgets, true, super::SUCCESS_MESSAGE);
        return;
    }

    let cmd = &commands[index];

    widgets.update_task_status(index, TaskStatus::Running);
    widgets.set_title(&cmd.description);

    let (program, args) = match resolve_command(cmd) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to prepare command: {}", err);
            let error_msg = format!("Failed to prepare command: {}\n", err);
            widgets.append_colored(&error_msg, "error");
            widgets.update_task_status(index, TaskStatus::Failed);
            finalize_execution(
                &widgets,
                false,
                &format!("Failed to prepare command: {}", err),
            );
            return;
        }
    };

    info!("Executing: {} {:?}", program, args);

    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::thread;

    let context = RunningContext::new(
        widgets.clone(),
        commands.clone(),
        index,
        cancelled.clone(),
        current_pid.clone(),
    );

    widgets.append_command_header(&cmd.description);

    let mut process = Command::new(&program);
    process.args(&args);

    // Inject sudo shim to intercept sudo calls in scripts
    let scripts_dir = crate::config::paths::scripts();
    if scripts_dir.exists() {
        if let Ok(path) = std::env::var("PATH") {
            let new_path = format!("{}:{}", scripts_dir.display(), path);
            process.env("PATH", new_path);
        }
    }

    process.stdout(Stdio::piped());
    process.stderr(Stdio::piped());

    let child = match process.spawn() {
        Ok(child) => child,
        Err(err) => {
            error!("Failed to start command: {}", err);
            let error_msg = format!("Failed to start operation: {}\n", err);
            widgets.append_colored(&error_msg, "error");
            widgets.update_task_status(index, TaskStatus::Failed);
            finalize_execution(
                &widgets,
                false,
                &format!("Failed to start operation: {}", err),
            );
            return;
        }
    };

    use std::sync::Mutex;
    *current_pid.borrow_mut() = Some(child.id());
    let child_arc = Arc::new(Mutex::new(Some(child)));

    let result_arc: Arc<Mutex<Option<CommandResult>>> = Arc::new(Mutex::new(None));

    use std::sync::mpsc;
    let (stdout_tx, stdout_rx) = mpsc::channel();
    let (stderr_tx, stderr_rx) = mpsc::channel();

    let stdout_handle = child_arc
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|c| c.stdout.take())
        .map(|stdout| {
            thread::spawn(move || {
                read_buffer_with_line_processing(
                    stdout,
                    |text| match stdout_tx.send(text) {
                        Ok(()) => true,
                        Err(e) => {
                            warn!("Failed to send stdout chunk to channel: {}", e);
                            false
                        }
                    },
                    |e| {
                        warn!("Error reading stdout: {}", e);
                    },
                );
            })
        });

    let stderr_handle = child_arc
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|c| c.stderr.take())
        .map(|stderr| {
            thread::spawn(move || {
                read_buffer_with_line_processing(
                    stderr,
                    |text| match stderr_tx.send(text) {
                        Ok(()) => true,
                        Err(e) => {
                            warn!("Failed to send stderr chunk to channel: {}", e);
                            false
                        }
                    },
                    |e| {
                        warn!("Error reading stderr: {}", e);
                    },
                );
            })
        });

    let widgets_stdout = widgets.clone();
    let widgets_stderr = widgets.clone();
    let result_arc_for_output = result_arc.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        while let Ok(text) = stdout_rx.try_recv() {
            let cleaned_text = strip_ansi_escapes::strip_str(&text);
            widgets_stdout.append_colored(&cleaned_text, "stdout");
        }
        while let Ok(text) = stderr_rx.try_recv() {
            let cleaned_text = strip_ansi_escapes::strip_str(&text);
            widgets_stderr.append_colored(&cleaned_text, "stderr");
        }
        if result_arc_for_output.lock().unwrap().is_some() {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    let result_arc_clone = result_arc.clone();

    thread::spawn(move || {
        if let Some(handle) = stdout_handle {
            if let Err(e) = handle.join() {
                warn!("Error joining stdout reader thread: {:?}", e);
            }
        }
        if let Some(handle) = stderr_handle {
            if let Err(e) = handle.join() {
                warn!("Error joining stderr reader thread: {:?}", e);
            }
        }

        let mut child_guard = child_arc.lock().unwrap();
        if let Some(mut child) = child_guard.take() {
            let result = match child.wait() {
                Ok(status) => {
                    if status.success() {
                        CommandResult::Success
                    } else {
                        CommandResult::Failure {
                            exit_code: status.code(),
                        }
                    }
                }
                Err(e) => {
                    error!("Error waiting for process: {}", e);
                    CommandResult::Failure { exit_code: None }
                }
            };
            *result_arc_clone.lock().unwrap() = Some(result);
        }
    });

    let context_clone = context.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let mut result_guard = result_arc.lock().unwrap();
        if let Some(result) = result_guard.take() {
            context_clone.set_exit_result(result);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

/// Resolve command to executable program and arguments,
/// handling privilege escalation (pkexec) and AUR helper detection.
///
/// # Returns
///
/// A tuple of `(program, args)` where `program` is the executable to run
/// and `args` are the arguments to pass to it.
///
/// # Errors
///
/// Returns an error if the AUR helper is required but not available.
fn resolve_command(command: &Command) -> Result<(String, Vec<String>), String> {
    let scripts_dir = crate::config::paths::scripts();
    let shim_path_env = if scripts_dir.exists() {
        if let Ok(path) = std::env::var("PATH") {
            Some(format!("PATH={}:{}", scripts_dir.display(), path))
        } else {
            None
        }
    } else {
        None
    };

    match command.command_type {
        CommandType::Normal => Ok((command.program.clone(), command.args.clone())),
        CommandType::Privileged => {
            // Use xero-auth client instead of pkexec for better session reuse
            let mut args = Vec::new();

            if let Some(env) = &shim_path_env {
                args.push("--env".to_string());
                args.push(env.clone());
            }

            args.push(command.program.clone());
            args.extend(command.args.clone());
            Ok((get_xero_auth_path().to_string_lossy().to_string(), args))
        }
        CommandType::Aur => {
            let helper = core::aur_helper()
                .ok_or_else(|| "AUR helper not available (paru or yay required)".to_string())?;
            let mut args = Vec::with_capacity(command.args.len() + 2);
            args.push("--sudo".to_string());
            args.push(get_xero_auth_path().to_string_lossy().to_string());
            args.extend(command.args.clone());
            Ok((helper.to_string(), args))
        }
    }
}

fn stop_daemon_if_needed() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(core::daemon::stop_daemon()) {
        error!("Failed to stop daemon: {}", e);
    }
}

/// Finalize dialog with success or failure message.
pub fn finalize_execution(widgets: &TaskRunnerWidgets, success: bool, message: &str) {
    use std::sync::atomic::Ordering;

    stop_daemon_if_needed();

    if success {
        let success_msg = format!("\n{}\n", message);
        widgets.append_colored(&success_msg, "stdout");
    } else {
        let error_msg = format!("\n{}\n", message);
        widgets.append_colored(&error_msg, "error");
    }

    super::ACTION_RUNNING.store(false, Ordering::SeqCst);
    widgets.show_completion(success, message);
}
