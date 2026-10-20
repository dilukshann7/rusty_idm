pub mod commands;
pub mod download_form;
pub mod duplicates;
pub mod lifecycle;
pub mod presenter;
pub mod progress;
pub mod settings;
pub mod transfer_view;

#[cfg(windows)]
mod app;
#[cfg(windows)]
pub mod bridge;
#[cfg(windows)]
pub mod browser_integration;
#[cfg(windows)]
mod completion;
#[cfg(windows)]
mod context_menu;
#[cfg(windows)]
mod controls;
#[cfg(windows)]
mod details;
#[cfg(windows)]
mod dialogs;
#[cfg(windows)]
pub mod download_intent;
#[cfg(windows)]
mod icons;
#[cfg(windows)]
pub mod platform;
#[cfg(windows)]
mod queue_ui;
#[cfg(windows)]
mod shell_icons;
#[cfg(windows)]
mod tray;

#[cfg(windows)]
pub use app::{run, AppError};
