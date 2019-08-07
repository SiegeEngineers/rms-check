const { RMSCheckServer } = require('./pkg/rms_check_vscode_server_wasm')
const { StreamMessageReader } = require('vscode-jsonrpc')

const inp = new StreamMessageReader(process.stdin)
// const out = new StreamMessageWriter(process.stdout)

global.write_message = (message) => {
  const len = Buffer.byteLength(message, 'utf8')
  process.stdout.write(`Content-Length: ${len}\r\n\r\n${message}`)
}

const server = new RMSCheckServer()

inp.listen((message) => {
  server.write(JSON.stringify(message))
})
