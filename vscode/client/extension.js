const path = require('path')
const { window, workspace } = require('vscode')
const { LanguageClient, TransportKind } = require('vscode-languageclient')

let client = null

exports.activate = function activate (context) {
  const server = path.join(__dirname, '../../target/debug/rms-check')

  const serverOptions = {
    run: {
      command: server,
      args: ['--server'],
      transport: TransportKind.stdio
    }
  }

  const clientOptions = {
    documentSelector: ['aoe2-rms']
  }

  client = new LanguageClient('rmsCheck', 'rms-check', serverOptions, clientOptions)

  client.start()
}

exports.deactivate = function deactivate () {
  if (client) {
    client.stop()
    client = null
  }
}
