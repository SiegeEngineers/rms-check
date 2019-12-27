const path = require('path')
const { TextDecoder, promisify } = require('util')
const { ZipFile } = require('yazl')
const zip = require('./store-zip')
const concat = promisify(require('simple-concat'))
const { commands, window, workspace, FileSystemError, FileType, Uri } = require('vscode')
const { LanguageClient, TransportKind } = require('vscode-languageclient')

let client = null
let decoder = null

const c = window.createOutputChannel('rms-check')

const globalConfig = workspace.getConfiguration('rmsCheck')
let storagePath = null

const major = process.version.match(/^v(\d+)/)[1]
const defaultUseWasm = parseInt(major, 10) >= 10

const useWasm = globalConfig.server === 'native' ? false
  : globalConfig.server === 'wasm' ? true
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

async function editZrMap (uri) {
  c.appendLine(`Edit ZR@ map: ${uri}`)
  const basename = path.basename(uri.fsPath)
  const bytes = await workspace.fs.readFile(uri)
  c.appendLine(`  readFile: ${bytes.length} bytes`)
  const files = zip.read(bytes)
  c.appendLine(`  ${files.length} files`)

  c.appendLine(`Add workspace folder: ${toZrUri(uri)}`)
  workspace.updateWorkspaceFolders(workspace.workspaceFolders.length, 0, { uri: toZrUri(uri), name: basename })

  const mainFile = files.find((f) => /\.rms$/.test(f.header.name))
  if (mainFile) {
    const doc = await workspace.openTextDocument(toZrUri(uri, mainFile.header.name))
    await window.showTextDocument(doc)
  }
}

exports.activate = function activate (context) {
  const serverOptions = useWasm ? getWasmServerOptions() : getNativeServerOptions()
  const clientOptions = {
    documentSelector: ['aoe2-rms']
  }

  decoder = new TextDecoder()
  storagePath = context.storagePath

  client = new LanguageClient('rmsCheck', 'rms-check', serverOptions, clientOptions)
  client.start()

  context.subscriptions.push(commands.registerCommand('rms-check.edit-zr-map', async (uri) => {
    try {
      await editZrMap(uri)
    } catch (err) {
      window.showErrorMessage(err.stack)
    }
  }))
  context.subscriptions.push(workspace.registerFileSystemProvider('aoe2-rms-zr', new ZipRmsFileSystemProvider(), {
    isCaseSensitive: true,
    isReadonly: false
  }))
}

function toZrUri (uri, filename = '') {
  return uri.with({ scheme: 'aoe2-rms-zr', path: `${uri.path}/${filename}` })
}
function toFileUri (uri) {
  let path = uri.path
  const lastSlash = path.lastIndexOf('/')
  const secondToLastSlash = path.lastIndexOf('/', lastSlash - 1)
  if (lastSlash !== -1 && secondToLastSlash !== -1) {
    const filename = path.slice(lastSlash + 1)
    path = path.slice(0, lastSlash)
    return [uri.with({ scheme: 'file', path }), filename]
  }
}

class ZipRmsFileSystemProvider {
  onDidChangeFile (listener) {
    c.appendLine('onDidChangeFile')
    // Ignore for now, should watch the zip file and check entry mtimes in the future
  }

  createDirectory () {
    throw FileSystemError.NoPermissions('ZR@-maps cannot contain directories')
  }

  async delete (uri, options) {
    c.appendLine(`Deleting file: ${uri}`)
    const [zipFile, filename] = toFileUri(uri)

    await this._editFile(zipFile, (files, newZip) => {
      for (const { data, header } of files) {
        if (header.name === filename) {
          continue
        }
        newZip.addBuffer(data, header.name, {
          mtime: fromDosDateTime(header.mdate, header.mtime),
          compress: false
        })
      }
    })
  }

  async readDirectory (uri) {
    c.appendLine(`Reading directory: ${uri}`)
    const [zipFile, filename] = toFileUri(uri)
    c.appendLine(`zipFile = ${zipFile}`)

    const bytes = await workspace.fs.readFile(zipFile)
    const files = zip.read(bytes)
    c.appendLine(`${files.length} files`)

    return files.map((f) => {
      return [f.header.name, FileType.File]
    })
  }

  async readFile (uri) {
    c.appendLine(`Reading file: ${uri}`)
    const [zipFile, filename] = toFileUri(uri)

    const bytes = await workspace.fs.readFile(zipFile)
    const files = zip.read(bytes)

    const file = files.find((f) => f.header.name === filename)
    if (!file) {
      throw FileSystemError.FileNotFound(uri)
    }

    return file.data
  }

  async rename (oldUri, newUri, options) {
    c.appendLine(`Renaming file: ${uri}`)
    const [oldZipFile, oldFilename] = toFileUri(oldUri)
    const [newZipFile, newFilename] = toFileUri(newUri)

    if (oldZipFile !== newZipFile) {
      throw new FileSystemError('Cannot move files between ZR@-maps')
    }

    await this._editFile(oldZipFile, (files, newZip) => {
      for (const { data, header } of files) {
        let name = header.name
        if (name === oldFilename) {
          mtime = new Date()
        }
        newZip.addBuffer(data, name, {
          mtime: fromDosDateTime(header.mdate, header.mtime),
          compress: false
        })
      }
    })
  }

  async stat (uri) {
    c.appendLine(`Stat ${uri}`)
    const [zipFile, filename] = toFileUri(uri)

    if (filename === '') {
      const stat = await workspace.fs.stat(zipFile)
      return {
        ctime: stat.ctime,
        mtime: stat.mtime,
        size: stat.size,
        type: FileType.Directory,
      }
    }

    const bytes = await workspace.fs.readFile(zipFile)
    const files = zip.read(bytes)

    const file = files.find((f) => f.header.name === filename)
    if (!file) {
      throw FileSystemError.FileNotFound(uri)
    }

    // TODO implement this part
    const mtime = fromDosDateTime(file.header.mdate, file.header.mtime)

    return {
      ctime: +mtime,
      mtime: +mtime,
      size: file.uncompressedSize,
      type: FileType.File
    }
  }

  watch (uri, options) {
    // throw FileSystemError.Unavailable('not yet implemented')
  }

  async writeFile (uri, content, options) {
    c.appendLine(`Writing file: ${uri}, ${content.length} bytes`)
    const [zipFile, filename] = toFileUri(uri)

    await this._editFile(zipFile, (files, newZip) => {
      for (const { data, header } of files) {
        if (header.name === filename) {
          continue
        }
        const buffer = Buffer.from(data.buffer, data.byteOffset, data.byteLength)
        newZip.addBuffer(Buffer.from(data), header.name, {
          mtime: fromDosDateTime(header.mdate, header.mtime),
          compress: false
        })
      }

      const buffer = Buffer.from(content.buffer, content.byteOffset, content.byteLength)
      newZip.addBuffer(buffer, filename, { compress: false })
    })
  }

  async _editFile (zipFile, edit) {
    const bytes = await workspace.fs.readFile(zipFile)
    const files = zip.read(bytes)
    const newZip = new ZipFile()
    const concatBytes = concat(newZip.outputStream)

    await edit(files, newZip)
    newZip.end()

    const buffer = await concatBytes
    const newBytes = new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength)

    await workspace.fs.writeFile(zipFile, newBytes)
  }
}

function toDosDateTime (date) {
  return [
    (date.getHours() << 11) + (date.getMinutes() << 5) + (date.getSeconds() / 2),
    ((date.getFullYear() - 1980) << 9) + ((date.getMonth() + 1) << 5) + date.getDate()
  ]
}

function fromDosDateTime (date, time) {
  const year = (date >> 9) + 1980
  const month = ((date >> 5) & 0xF)
  const day = (date & 0x1F)
  const hour = (time >> 11)
  const min = ((time >> 5) & 0x3F)
  const sec = (time & 0x1F) * 2

  return new Date(year, month, day, hour, min, sec)
}

exports.deactivate = function deactivate () {
  if (client) {
    client.stop()
    client = null
  }
}
