const path = require('path')
const { window, workspace } = require('vscode')
const { LanguageClient, TransportKind } = require('vscode-languageclient')

let client = null

const configuration = workspace.getConfiguration('rmsCheck')

const major = process.version.match(/^v(\d+)/)[1]
const defaultUseWasm = parseInt(major, 10) >= 10

const useWasm = configuration.server === 'native' ? false
  : configuration.server === 'wasm' ? true
  : defaultUseWasm

function getWasmServerOptions () {
  return {
    run: {
      module: require.resolve('../server'),
      transport: TransportKind.stdio
    }
  }
}

function getNativeServerOptions () {
  let localServer = path.join(__dirname, '../../target/debug/rms-check')
  try {
    fs.accessSync(localServer)
  } catch (err) {
    localServer = null
  }

  return {
    run: {
      command: 'rms-check',
      args: ['server'],
      transport: TransportKind.stdio
    },
    debug: localServer && {
      command: localServer,
      args: ['server'],
      transport: TransportKind.stdio
    }
  }
}

exports.activate = function activate (context) {
  const serverOptions = useWasm ? getWasmServerOptions() : getNativeServerOptions()
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
