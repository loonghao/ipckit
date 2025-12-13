//! Serve command implementation

use super::{print_info, print_success};
use ipckit::socket_server::{Connection, FnHandler, Message, SocketServer, SocketServerConfig};
use ipckit::task_manager::{TaskManager, TaskManagerConfig};

pub fn serve(
    socket: Option<String>,
    _port: Option<u16>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = socket.unwrap_or_else(|| {
        #[cfg(windows)]
        {
            "\\\\.\\pipe\\ipckit".to_string()
        }
        #[cfg(unix)]
        {
            "/tmp/ipckit.sock".to_string()
        }
    });

    print_info(&format!("Starting API server on {}", socket_path));

    // Create task manager
    let _task_manager = TaskManager::new(TaskManagerConfig::default());

    // Create socket server config
    let config = SocketServerConfig::with_path(&socket_path);

    // Create and run server
    let server = SocketServer::new(config)?;

    print_success(&format!("API server listening on {}", socket_path));

    if verbose {
        println!("Available endpoints:");
        println!("  GET  /v1/tasks          - List all tasks");
        println!("  GET  /v1/tasks/{{id}}     - Get task by ID");
        println!("  POST /v1/tasks          - Create a new task");
        println!("  DELETE /v1/tasks/{{id}}  - Cancel a task");
        println!("  GET  /v1/health         - Health check");
    }

    println!("Press Ctrl+C to stop...");

    // Simple handler that responds to messages
    server.run(FnHandler::new(|_conn: &mut Connection, msg: Message| {
        // Get the request content
        let request = if let Some(binary) = msg.as_binary() {
            String::from_utf8_lossy(&binary).to_string()
        } else if let Some(text) = msg.as_text() {
            text.to_string()
        } else {
            serde_json::to_string(&msg.payload).unwrap_or_default()
        };

        // Simple routing
        let response = if request.contains("GET /v1/health") {
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}"
        } else if request.contains("GET /v1/tasks") {
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n[]"
        } else {
            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\n\r\n{\"error\":\"Not Found\"}"
        };

        Ok(Some(Message::binary(response.as_bytes().to_vec())))
    }))?;

    Ok(())
}
