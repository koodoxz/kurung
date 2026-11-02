import { execFile } from 'child_process';
import readline from 'readline';

const KURUNG_BIN = 'C:/Pato/Nusantara-Rust-Suite/kurung/target/release/kurung.exe';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

rl.on('line', (line) => {
  if (!line.trim()) return;
  try {
    const req = JSON.parse(line);
    handleRequest(req);
  } catch (err) {
    send({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
  }
});

function handleRequest(req) {
  const { id, method, params } = req;

  if (method === 'initialize') {
    return send({
      jsonrpc: '2.0',
      id,
      result: {
        protocolVersion: '2024-11-05',
        capabilities: { tools: {} },
        serverInfo: { name: 'kurung-mcp', version: '0.1.0' },
      },
    });
  }

  if (method === 'notifications/initialized') {
    return;
  }

  if (method === 'tools/list') {
    return send({
      jsonrpc: '2.0',
      id,
      result: {
        tools: [
          {
            name: 'kurung_run_wasm',
            description: 'Execute a WebAssembly module (.wat or .wasm) inside the Kurung deterministic sandbox with fuel & memory caps.',
            inputSchema: {
              type: 'object',
              properties: {
                file_path: { type: 'string', description: 'Path to .wat or .wasm file' },
                entry: { type: 'string', description: 'Exported function name', default: 'run' },
                input: { type: 'string', description: 'Payload string', default: '' },
                max_fuel: { type: 'number', description: 'Instruction budget', default: 500000 },
                max_memory_mb: { type: 'number', description: 'RAM limit in MiB', default: 16 },
              },
              required: ['file_path'],
            },
          },
          {
            name: 'kurung_audit_self',
            description: 'Execute the built-in 4-stage adversarial verification suite.',
            inputSchema: { type: 'object', properties: {} },
          },
        ],
      },
    });
  }

  if (method === 'tools/call') {
    const toolName = params?.name;
    const args = params?.arguments || {};

    if (toolName === 'kurung_audit_self') {
      execFile(KURUNG_BIN, ['audit-self'], (err, stdout, stderr) => {
        if (err) {
          return send({
            jsonrpc: '2.0',
            id,
            result: { isError: true, content: [{ type: 'text', text: stderr || err.message }] },
          });
        }
        return send({
          jsonrpc: '2.0',
          id,
          result: { content: [{ type: 'text', text: stdout }] },
        });
      });
      return;
    }

    if (toolName === 'kurung_run_wasm') {
      const filePath = String(args.file_path || '');
      const entry = String(args.entry || 'run');
      const input = String(args.input || '');
      const maxFuel = String(args.max_fuel || 500000);
      const maxMem = String(args.max_memory_mb || 16);

      execFile(
        KURUNG_BIN,
        ['run', filePath, '--entry', entry, '--input', input, '--max-fuel', maxFuel, '--max-memory-mb', maxMem, '--json'],
        (err, stdout, stderr) => {
          if (err) {
            return send({
              jsonrpc: '2.0',
              id,
              result: { isError: true, content: [{ type: 'text', text: stderr || err.message }] },
            });
          }
          return send({
            jsonrpc: '2.0',
            id,
            result: { content: [{ type: 'text', text: stdout }] },
          });
        }
      );
      return;
    }

    return send({
      jsonrpc: '2.0',
      id,
      error: { code: -32601, message: `Tool not found: ${toolName}` },
    });
  }

  return send({
    jsonrpc: '2.0',
    id,
    error: { code: -32601, message: `Method not supported: ${method}` },
  });
}
