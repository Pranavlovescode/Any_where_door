use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

const SERVICE_NAME: &str = "AnywhereDoorAgent";

define_windows_service!(ffi_service_main, service_main);

pub fn run_dispatcher() -> Result<(), String> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|err| format!("Failed to start Windows service dispatcher: {err}"))
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(err) = run_service_main() {
        eprintln!("Windows service failed: {err}");
    }
}

fn run_service_main() -> Result<(), String> {
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_requested_for_handler = Arc::clone(&stop_requested);

    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop => {
                stop_requested_for_handler.store(true, Ordering::SeqCst);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })
    .map_err(|err| format!("Failed to register service control handler: {err}"))?;

    set_status(
        &status_handle,
        ServiceState::StartPending,
        ServiceExitCode::Win32(0),
    )?;
    set_status(
        &status_handle,
        ServiceState::Running,
        ServiceExitCode::Win32(0),
    )?;

    let run_result = crate::service::run_loop(stop_requested, false, false);

    let stopped_exit_code = if run_result.is_ok() {
        ServiceExitCode::Win32(0)
    } else {
        ServiceExitCode::Win32(1)
    };

    set_status(
        &status_handle,
        ServiceState::StopPending,
        ServiceExitCode::Win32(0),
    )?;
    set_status(&status_handle, ServiceState::Stopped, stopped_exit_code)?;

    run_result
}

fn set_status(
    status_handle: &service_control_handler::ServiceStatusHandle,
    current_state: ServiceState,
    exit_code: ServiceExitCode,
) -> Result<(), String> {
    let controls_accepted = match current_state {
        ServiceState::Running => ServiceControlAccept::STOP,
        _ => ServiceControlAccept::empty(),
    };

    let wait_hint = match current_state {
        ServiceState::StartPending | ServiceState::StopPending => Duration::from_secs(10),
        _ => Duration::default(),
    };

    let status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted,
        exit_code,
        checkpoint: 0,
        wait_hint,
        process_id: None,
    };

    status_handle
        .set_service_status(status)
        .map_err(|err| format!("Failed to set service status: {err}"))
}