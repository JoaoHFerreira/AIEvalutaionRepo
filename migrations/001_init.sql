CREATE TABLE results (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL
);

INSERT INTO results (title, description) VALUES
('Rust Programming', 'A language empowering everyone to build reliable and efficient software.'),
('WASM in Action', 'WebAssembly is a binary instruction format for a stack-based virtual machine.'),
('Docker Compose', 'Define and run multi-container applications with Docker.');
