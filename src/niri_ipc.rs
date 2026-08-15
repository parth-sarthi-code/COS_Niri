use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Request, Response, Window, Workspace, WorkspaceReferenceArg};

pub type NiriWorkspace = Workspace;

pub struct NiriIpcClient;

impl NiriIpcClient {
    /// Fetch initial workspaces snapshot.
    pub fn get_workspaces() -> Result<Vec<Workspace>, Box<dyn std::error::Error>> {
        let mut socket = Socket::connect()?;
        let response = socket.send(Request::Workspaces)?;
        match response {
            Ok(Response::Workspaces(workspaces)) => Ok(workspaces),
            Ok(Response::Handled) => Err("Unexpected response: Handled".into()),
            Ok(_) => Err("Unexpected IPC response".into()),
            Err(e) => Err(format!("IPC Error: {e}").into()),
        }
    }

    /// Fetch initial open windows snapshot.
    pub fn get_windows() -> Result<Vec<Window>, Box<dyn std::error::Error>> {
        let mut socket = Socket::connect()?;
        let response = socket.send(Request::Windows)?;
        match response {
            Ok(Response::Windows(windows)) => Ok(windows),
            Ok(Response::Handled) => Err("Unexpected response: Handled".into()),
            Ok(_) => Err("Unexpected IPC response".into()),
            Err(e) => Err(format!("IPC Error: {e}").into()),
        }
    }



    /// Subscribe to Niri's EventStream.
    /// Blocks the current thread and feeds events to `on_event`.
    pub fn listen_events<F>(mut on_event: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(Event) + Send + 'static,
    {
        let mut socket = Socket::connect()?;
        let response = socket.send(Request::EventStream)?;
        if !matches!(response, Ok(Response::Handled)) {
            return Err("Failed to subscribe to EventStream".into());
        }

        let mut read_event = socket.read_events();
        while let Ok(event) = read_event() {
            on_event(event);
        }

        Ok(())
    }

    /// Asynchronously request workspace focus by workspace ID so GTK main loop never blocks.
    pub fn focus_workspace_id(id: u64) {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(mut socket) = Socket::connect() {
                let action = Action::FocusWorkspace {
                    reference: WorkspaceReferenceArg::Id(id),
                };
                let _ = socket.send(Request::Action(action));
            }
        });
    }

    /// Asynchronously request workspace focus by 1-based index.
    pub fn focus_workspace_index(idx: u8) {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(mut socket) = Socket::connect() {
                let action = Action::FocusWorkspace {
                    reference: WorkspaceReferenceArg::Index(idx),
                };
                let _ = socket.send(Request::Action(action));
            }
        });
    }

    /// Focus a window by ID.
    pub fn focus_window(id: u64) {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(mut socket) = Socket::connect() {
                let action = Action::FocusWindow { id };
                let _ = socket.send(Request::Action(action));
            }
        });
    }
}
