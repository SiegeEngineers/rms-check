const path = require('path')
const { TextDecoder } = require('util')
const zip = require('./store-zip')
const { commands, window, workspace } = require('vscode')
const { LanguageClient, TransportKind } = require('vscode-languageclient')

let client = null
let decoder = null

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

const openedZrMaps = new Map()

async function editZrMap (uri) {
  const file = uri.fsPath

  const panel = window.createWebviewPanel('rms-check.zr-map', path.basename(file), -1, {
    enableFindWidget: true,
    enableCommandUris: true,
    enableScripts: true
  })

  const bytes = await workspace.fs.readFile(uri)
  const files = zip.read(bytes)
  openedZrMaps.set(uri.toString(true), files)

  const mainFile = files.find((f) => /\.rms$/.test(f.header.name))
  if (mainFile) {
    workspace.openTextDocument(uri.with({ fragment: mainFile.header.name, scheme: 'aoe2-rms-zr' }))
  }

  panel.webview.html = `
    <!DOCTYPE html>
    <body>
      <ul>
        ${files.map(f => `<li>${f.header.name}</li>`).join('')}
      </ul>
    </body>
  `
}

exports.activate = function activate (context) {
  const serverOptions = useWasm ? getWasmServerOptions() : getNativeServerOptions()
  const clientOptions = {
    documentSelector: ['aoe2-rms']
  }

  decoder = new TextDecoder()

  client = new LanguageClient('rmsCheck', 'rms-check', serverOptions, clientOptions)
  client.start()

  context.subscriptions.push(commands.registerCommand('rms-check.edit-zr-map', editZrMap))
  context.subscriptions.push(workspace.registerTextDocumentContentProvider('aoe2-rms-zr', {
    provideTextDocumentContent (uri, cancelToken) {
      const zrUri = uri.with({
        fragment: null,
        scheme: 'file'
      })
      const zr = openedZrMaps.get(zrUri.toString(true))
      if (!zr) {
        throw new Error('ZR@ map was not opened.')
      }

      const file = zr.find((f) => f.header.name == uri.fragment)
      if (!file) {
        throw new Error('ZR@ map does not contain a .rms file.')
      }

      return decoder.decode(file.data)
    }
  }))
}

exports.deactivate = function deactivate () {
  if (client) {
    client.stop()
    client = null
  }
}
