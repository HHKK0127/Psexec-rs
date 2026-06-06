use super::protocol::{ExecutionSettings, ExecutionResult};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct InteractiveSession {
    #[allow(dead_code)]
    target_host: String,
    commands: VecDeque<String>,
    results: Vec<ExecutionResult>,
    is_active: bool,
    output_buffer: Arc<Mutex<Vec<String>>>,
}

impl InteractiveSession {
    pub fn new(target_host: &str) -> Self {
        InteractiveSession {
            target_host: target_host.to_string(),
            commands: VecDeque::new(),
            results: Vec::new(),
            is_active: false,
            output_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        // In production, this would establish a Named Pipe connection
        // For now, mark as active
        self.is_active = true;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.is_active = false;
        Ok(())
    }

    pub fn queue_command(&mut self, command: &str) {
        if command.to_lowercase() == "exit" {
            self.is_active = false;
        } else {
            self.commands.push_back(command.to_string());
        }
    }

    pub fn queue_multiple_commands(&mut self, commands: &[&str]) {
        for cmd in commands {
            self.queue_command(cmd);
        }
    }

    pub fn execute_queued(&mut self) -> Result<Vec<ExecutionResult>, String> {
        if !self.is_active {
            return Err("Session is not active".to_string());
        }

        while let Some(command) = self.commands.pop_front() {
            let settings = ExecutionSettings {
                command,
                working_directory: None,
                priority: None,
                env_vars: None,
            };

            let result = self.execute_command(&settings)?;
            self.results.push(result);
        }

        Ok(self.results.clone())
    }

    pub fn execute_interactive_loop(&mut self) -> Result<(), String> {
        if !self.is_active {
            return Err("Session is not active".to_string());
        }

        // In production, this would:
        // 1. Read from stdin
        // 2. Send to remote named pipe
        // 3. Read response
        // 4. Print output
        // 5. Loop until "exit" is entered

        Ok(())
    }

    fn execute_command(&self, settings: &ExecutionSettings) -> Result<ExecutionResult, String> {
        // In production, this would:
        // 1. Serialize settings to JSON
        // 2. Create Message with MessageType::Settings
        // 3. Send over Named Pipe
        // 4. Receive response
        // 5. Deserialize ExecutionResult

        // For now, mock implementation
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: format!("Executed: {}", settings.command),
            stderr: String::new(),
        })
    }

    pub fn get_output(&self) -> Vec<String> {
        self.output_buffer.lock().unwrap().clone()
    }

    pub fn append_output(&self, line: &str) {
        self.output_buffer.lock().unwrap().push(line.to_string());
    }

    pub fn get_results(&self) -> &[ExecutionResult] {
        &self.results
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn pending_commands(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_session_creation() {
        let session = InteractiveSession::new("localhost");
        assert!(!session.is_active());
        assert_eq!(session.pending_commands(), 0);
    }

    #[test]
    fn test_command_queueing() {
        let mut session = InteractiveSession::new("localhost");
        session.queue_command("whoami");
        session.queue_command("ipconfig");

        assert_eq!(session.pending_commands(), 2);
    }

    #[test]
    fn test_exit_command() {
        let mut session = InteractiveSession::new("localhost");
        session.is_active = true;
        session.queue_command("whoami");
        session.queue_command("exit");

        assert!(!session.is_active());
    }

    #[test]
    fn test_output_buffer() {
        let session = InteractiveSession::new("localhost");
        session.append_output("line 1");
        session.append_output("line 2");

        let output = session.get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "line 1");
    }

    #[test]
    fn test_multiple_command_queueing() {
        let mut session = InteractiveSession::new("localhost");
        let cmds = vec!["cmd1", "cmd2", "cmd3"];
        session.queue_multiple_commands(&cmds);

        assert_eq!(session.pending_commands(), 3);
    }
}
