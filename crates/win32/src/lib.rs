pub mod commands;
pub mod download_form;
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
mod controls;
#[cfg(windows)]
mod details;
#[cfg(windows)]
mod dialogs;
#[cfg(windows)]
mod icons;
#[cfg(windows)]
pub mod platform;

#[cfg(windows)]
pub use app::{run, AppError};
