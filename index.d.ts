// Type definitions for @jscpd/fs-extra (the fs-extra 11.4.1 API)
// A drop-in replacement for fs-extra, implemented natively (napi-rs/Rust).
// Mirrors the fs-extra 11.x API: every method in its sync, callback and
// promise forms, plus the re-exported fs surface.

import * as fs from 'node:fs'
import * as events from 'node:events'

declare const __universalifyMethods: unique symbol

export type JsonPrimitive = string | number | boolean | null
export interface JsonObject extends Record<string, JsonValue> {}
export interface JsonArray extends Array<JsonValue> {}
export type JsonValue = JsonPrimitive | JsonObject | JsonArray

export type FilterFn = (src: string, dest: string) => boolean | Promise<boolean>

export interface FsFn {
  (src: string | Buffer, dest: string | Buffer, callback: (err: Error) => void): void
  (src: string | Buffer, dest: string | Buffer, options?: object | string | null): Promise<void>
}

export interface CopyOptions {
  mode?: number
  clobber?: boolean
  overwrite?: boolean
  preserveTimestamps?: boolean
  errorOnExist?: boolean
  dereference?: boolean
  /** Function that gets called for every item processed. */
  filter?: FilterFn
}

export interface MoveOptions {
  overwrite?: boolean
  clobber?: boolean
  dereference?: boolean
}

export interface EnsureDirOptions {
  mode?: number
}

export interface RmOptions {
  force?: boolean
  maxRetries?: number
  recursive?: boolean
  retryDelay?: number
}

export interface SymlinkOptions {
  type?: 'dir' | 'file' | 'junction'
}

export interface ReadOptions extends fs.ReadFileSyncOptions {}

export interface JsonReadOptions extends ReadOptions {
  throws?: boolean
  reviver?: (this: any, key: string, value: any) => any
}

export interface WriteFileBaseOptions extends Partial<fs.WriteFileOptions> {}

export type WriteFileOptions =
  | (WriteFileBaseOptions & { encoding?: fs.BufferEncoding | null, mode?: number | string, flag?: string })
  | fs.BufferEncoding
  | undefined

export interface JsonWriteFileOptions extends WriteFileBaseOptions {
  spaces?: number | string
  EOL?: string
  finalEOL?: boolean
  replacer?: number | string | (number | string)[] | null
}

// ---------------------------------------------------------------------------
// fs surface (re-exported — fs-extra is a drop-in replacement for graceful-fs)
// ---------------------------------------------------------------------------

export const F_OK: number
export const R_OK: number
export const W_OK: number
export const X_OK: number
export const constants: typeof fs.constants
export const Dir: typeof fs.Dir
export const Dirent: typeof fs.Dirent
export const FSWatcher: typeof fs.FSWatcher
export const ReadStream: typeof fs.ReadStream
export const WriteStream: typeof fs.WriteStream
export const Stats: typeof fs.Stats
export const FileHandle: typeof fs.promises.FileHandle
export default {
  copy,
  copySync,
  emptyDir,
  emptyDirSync,
  emptydir,
  emptydirSync,
  createFile,
  createFileSync,
  ensureFile,
  ensureFileSync,
  createLink,
  createLinkSync,
  ensureLink,
  ensureLinkSync,
  createSymlink,
  createSymlinkSync,
  ensureSymlink,
  ensureSymlinkSync,
  readJson,
  readJsonSync,
  readJSON,
  readJSONSync,
  writeJson,
  writeJsonSync,
  writeJSON,
  writeJSONSync,
  outputJson,
  outputJsonSync,
  outputJSON,
  outputJSONSync,
  mkdirs,
  mkdirsSync,
  mkdirp,
  mkdirpSync,
  ensureDir,
  ensureDirSync,
  move,
  moveSync,
  outputFile,
  outputFileSync,
  pathExists,
  pathExistsSync,
  remove,
  removeSync
}

// fs functions are re-exported from graceful-fs: the sync forms, the
// callback/promise universalified forms, streams, and fs.promises.
export const access: {
  (path: fs.PathLike, mode?: number, callback: (err: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, mode?: number): Promise<void>
}
export const appendFile: typeof fs.__promisify__ & ((file: fs.PathLike, data: string | Uint8Array, options?: WriteFileOptions | ((err: Error) => void), callback?: (err: Error) => void) => any)
export const chmod: {
  (path: fs.PathLike, mode: number | string, callback: (err: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, mode: number | string): Promise<void>
}
export const chown: {
  (path: fs.PathLike, uid: number, gid: number, callback: (err: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, uid: number, gid: number): Promise<void>
}
export const close: {
  (fd: number, callback: (err?: NodeJS.ErrnoException) => void): void
  (fd: number): Promise<void>
}
export const copyFile: {
  (src: fs.PathLike, dest: fs.PathLike, mode?: number, callback?: (err: NodeJS.ErrnoException) => void): void
  (src: fs.PathLike, dest: fs.PathLike, mode?: number): Promise<void>
}
export const cp: {
  (source: fs.PathLike, destination: fs.PathLike, opts?: fs.CopyOptions, callback?: (err: NodeJS.ErrnoException) => void): void
  (source: fs.PathLike, destination: fs.PathLike, opts?: fs.CopyOptions): Promise<void>
}
export const createReadStream: typeof fs.createReadStream
export const createWriteStream: typeof fs.createWriteStream
export const exists: {
  (path: fs.PathLike, callback: (exists: boolean) => void): void
  (path: fs.PathLike): Promise<boolean>
}
export const fchmod: {
  (fd: number, mode: number | string, callback: (err?: NodeJS.ErrnoException) => void): void
  (fd: number, mode: number | string): Promise<void>
}
export const fchown: {
  (fd: number, uid: number, gid: number, callback: (err?: NodeJS.ErrnoException) => void): void
  (fd: number, uid: number, gid: number): Promise<void>
}
export const fdatasync: {
  (fd: number, callback: (err?: NodeJS.ErrnoException) => void): void
  (fd: number): Promise<void>
}
export const fstat: {
  (fd: number, callback: (err: NodeJS.ErrnoException, stats: fs.Stats) => any): void
  (fd: number): Promise<fs.Stats>
}
export const fsync: {
  (fd: number, callback: (err?: NodeJS.ErrnoException) => void): void
  (fd: number): Promise<void>
}
export const ftruncate: {
  (fd: number, len: number | null, callback?: (err?: NodeJS.ErrnoException) => void): void
  (fd: number, len: number | null): Promise<void>
}
export const futimes: {
  (fd: number, atime: number | Date | string, mtime: number | Date | string, callback?: (err?: NodeJS.ErrnoException) => void): void
  (fd: number, atime: number | Date | string, mtime: number | Date | string): Promise<void>
}
export const lchmod: {
  (path: fs.PathLike, mode: number | string, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, mode: number | string): Promise<void>
}
export const lchown: {
  (path: fs.PathLike, uid: number, gid: number, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, uid: number, gid: number): Promise<void>
}
export const link: {
  (existingPath: fs.PathLike, newPath: fs.PathLike, callback: (err?: NodeJS.ErrnoException) => void): void
  (existingPath: fs.PathLike, newPath: fs.PathLike): Promise<void>
}
export const lstat: {
  (path: fs.PathLike, options?: fs.BigIntOptions | fs.StatOptions, callback?: (err: NodeJS.ErrnoException, stats: fs.Stats | fs.BigIntStats) => any): any
  (path: fs.PathLike, options?: fs.BigIntOptions | fs.StatOptions): Promise<fs.Stats | fs.BigIntStats>
}
export const lutimes: {
  (path: fs.PathLike, atime: number | Date | string, mtime: number | Date | string, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, atime: number | Date | string, mtime: number | Date | string): Promise<void>
}
export const mkdir: {
  (path: fs.PathLike, options?: fs.MkdirOptions | number | fs.Mode | null | undefined, callback?: (err: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, options?: fs.MkdirOptions | number | fs.Mode | null | undefined): Promise<string | undefined>
}
export const mkdirSync: (path: fs.PathLike, options?: fs.MkdirSyncOptions) => string | undefined
export const mkdtemp: {
  (prefix: string, options?: fs.EncodingOption, callback?: (err: NodeJS.ErrnoException, folder: string) => any): any
  (prefix: string, options?: fs.EncodingOption): Promise<string>
}
export const mkdtempSync: (prefix: string, options?: fs.EncodingOption | fs.BufferEncoding) => string
export const open: {
  (path: fs.PathLike, flags: string | number, mode?: number | string | null, callback?: (err: NodeJS.ErrnoException, fd: number) => any): any
  (path: fs.PathLike, flags: string | number, mode?: number | string | null): Promise<number>
}
export const openSync: (path: fs.PathLike, flags: string | number, mode?: fs.Mode) => number
export const opendir: {
  (path: fs.PathLike, options?: fs.OpenDirOptions, callback?: (err: NodeJS.ErrnoException, dir: fs.Dir) => void): void
  (path: fs.PathLike, options?: fs.OpenDirOptions): Promise<fs.Dir>
}
export const opendirSync: (path: fs.PathLike, options?: fs.OpenDirOptions) => fs.Dir
export const read: {
  (fd: number, buffer: Buffer | Uint8Array, offset: number, length: number, position: number | null, callback: (err: NodeJS.ErrnoException, bytesRead: number, buffer: Buffer | Uint8Array) => void): void
  (fd: number, buffer: Buffer | Uint8Array, offset: number, length: number, position?: number | null | undefined): Promise<{ bytesRead: number, buffer: Buffer | Uint8Array }>
}
export const readSync: (fd: number, buffer: Buffer | Uint8Array, offset: number, length: number, position: number | null) => number
export const readv: {
  (fd: number, buffers: readonly (Buffer | Uint8Array)[], position?: number | null | undefined, callback?: (err: NodeJS.ErrnoException, bytesRead: number, buffers: readonly (Buffer | Uint8Array)[]) => void): void
  (fd: number, buffers: readonly (Buffer | Uint8Array)[], position?: number | null | undefined): Promise<{ bytesRead: number, buffers: readonly (Buffer | Uint8Array)[] }>
}
export const readdir: {
  (path: fs.PathLike, callback?: (err: NodeJS.ErrnoException, files: string[]) => void): void
  (path: fs.PathLike, options: { withFileTypes: true, encoding?: BufferEncoding | null, recursive?: boolean }, callback?: (err: NodeJS.ErrnoException, files: fs.Dirent[]) => void): void
  (path: fs.PathLike, options: { withFileTypes: true, encoding?: BufferEncoding | null, recursive?: boolean }): Promise<fs.Dirent[]>
  (path: fs.PathLike, options?: ({ encoding?: BufferEncoding | null | undefined, recursive?: boolean, withFileTypes?: false | undefined } & fs.Abortable) | BufferEncoding | null): Promise<string[]>
}
export const readdirSync: {
  (path: fs.PathLike, options: { encoding: BufferEncoding | null, withFileTypes: true, recursive?: boolean }): fs.Dirent[]
  (path: fs.PathLike, options?: { encoding?: BufferEncoding | null, withFileTypes?: false, recursive?: boolean } | BufferEncoding | null): string[]
}
export const readFile: {
  (path: fs.PathLike | number, callback: (err: NodeJS.ErrnoException, data: Buffer) => void): void
  (path: fs.PathLike | number, options: { encoding: BufferEncoding, flag?: string } | BufferEncoding, callback: (err: NodeJS.ErrnoException, data: string) => void): void
  (path: fs.PathLike | number, options?: fs.ReadFileSyncOptions | fs.BufferEncoding | null): Promise<Buffer | string>
}
export const readFileSync: typeof fs.readFileSync
export const readlink: {
  (path: fs.PathLike, callback: (err: NodeJS.ErrnoException, linkString: string) => any): void
  (path: fs.PathLike, options?: { encoding?: BufferEncoding | null } & fs.Abortable | BufferEncoding | null, callback?: (err: NodeJS.ErrnoException, linkString: string | Buffer) => any): any
  (path: fs.PathLike, options?: { encoding?: BufferEncoding | null } | BufferEncoding | null): Promise<string>
  (path: fs.PathLike, options: { encoding: 'buffer' }): Promise<Buffer>
}
export const readlinkSync: typeof fs.readlinkSync
export const realpath: {
  (path: fs.PathLike, options?: fs.EncodingOption, callback?: (err: NodeJS.ErrnoException, resolvedPath: string) => any): any
  (path: fs.PathLike, options?: fs.EncodingOption): Promise<string>
  native: {
    (path: fs.PathLike, options?: fs.EncodingOption, callback?: (err: NodeJS.ErrnoException, resolvedPath: string) => any): any
    (path: fs.PathLike, options?: fs.EncodingOption): Promise<string>
  }
}
export const realpathSync: typeof fs.realpathSync
export const rename: {
  (oldPath: fs.PathLike, newPath: fs.PathLike, callback: (err?: NodeJS.ErrnoException) => void): void
  (oldPath: fs.PathLike, newPath: fs.PathLike): Promise<void>
}
export const renameSync: typeof fs.renameSync
export const rm: {
  (path: fs.PathLike, options?: RmOptions, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, options?: RmOptions): Promise<void>
}
export const rmSync: (path: fs.PathLike, options?: RmOptions) => void
export const rmdir: {
  (path: fs.PathLike, callback: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, options?: { maxRetries?: number, recursive?: boolean, retryDelay?: number }, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, options?: { maxRetries?: number, recursive?: boolean, retryDelay?: number }): Promise<void>
}
export const rmdirSync: typeof fs.rmdirSync
export const stat: {
  (path: fs.PathLike, callback: (err: NodeJS.ErrnoException, stats: fs.Stats) => any): void
  (path: fs.PathLike, options?: fs.StatOptions, callback?: (err: NodeJS.ErrnoException, stats: fs.Stats | fs.BigIntStats) => any): any
  (path: fs.PathLike, options?: fs.StatOptions): Promise<fs.Stats | fs.BigIntStats>
}
export const statSync: typeof fs.statSync
export const symlink: {
  (target: fs.PathLike, path: fs.PathLike, callback: (err?: NodeJS.ErrnoException) => void): void
  (target: fs.PathLike, path: fs.PathLike, type?: 'dir' | 'file' | 'junction' | SymlinkOptions, callback?: (err?: NodeJS.ErrnoException) => void): void
  (target: fs.PathLike, path: fs.PathLike, type?: 'dir' | 'file' | 'junction'): Promise<void>
}
export const symlinkSync: typeof fs.symlinkSync
export const truncate: {
  (path: fs.PathLike, len?: number | null, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, len?: number | null): Promise<void>
}
export const truncateSync: typeof fs.truncateSync
export const unlink: {
  (path: fs.PathLike, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike): Promise<void>
}
export const unlinkSync: typeof fs.unlinkSync
export const unwatchFile: typeof fs.unwatchFile
export const utimes: {
  (path: fs.PathLike, atime: number | string | Date, mtime: number | string | Date, callback?: (err?: NodeJS.ErrnoException) => void): void
  (path: fs.PathLike, atime: number | string | Date, mtime: number | string | Date): Promise<void>
}
export const utimesSync: typeof fs.utimesSync
export const watch: typeof fs.watch
export const watchFile: typeof fs.watchFile
export const write: {
  (fd: number, buffer: Buffer | Uint8Array, offset?: number | null | undefined, length?: number | null | undefined, position?: number | null | undefined, callback?: (err: NodeJS.ErrnoException, bytesWritten: number, buffer: Buffer | Uint8Array) => void): void
  (fd: number, buffer: Buffer | Uint8Array, offset?: number | null | undefined, length?: number | null | undefined, position?: number | null | undefined): Promise<{ bytesWritten: number, buffer: Buffer | Uint8Array }>
  (fd: number, string: string, position?: number | null | undefined, encoding?: BufferEncoding | null | undefined, callback?: (err: NodeJS.ErrnoException, written: number, string: string) => void): void
  (fd: number, string: string, position?: number | null | undefined, encoding?: BufferEncoding | null | undefined): Promise<{ bytesWritten: number, buffer: string }>
}
export const writeSync: typeof fs.writeSync
export const writev: {
  (fd: number, buffers: readonly (Buffer | Uint8Array)[], position?: number | null | undefined, callback?: (err?: NodeJS.ErrnoException, bytesWritten: number, buffers: readonly (Buffer | Uint8Array)[]) => void): void
  (fd: number, buffers: readonly (Buffer | Uint8Array)[], position?: number | null | undefined): Promise<{ bytesWritten: number, buffers: readonly (Buffer | Uint8Array)[] }>
}
export const writevSync: typeof fs.writevSync
export const writeFile: {
  (file: fs.PathLike | number, data: string | Uint8Array, callback: (err?: NodeJS.ErrnoException) => void): void
  (file: fs.PathLike | number, data: string | Uint8Array, options?: WriteFileOptions, callback?: (err?: NodeJS.ErrnoException) => void): void
  (file: fs.PathLike | number, data: string | Uint8Array, options?: WriteFileOptions): Promise<void>
}
export const writeFileSync: typeof fs.writeFileSync
export const promises: typeof fs.promises

// ---------------------------------------------------------------------------
// fs-extra methods
// ---------------------------------------------------------------------------

export function copy (
  src: string | Buffer,
  dest: string | Buffer,
  options?: CopyOptions | FilterFn,
  callback?: (err: Error) => void
): Promise<void>
export function copy (src: string | Buffer, dest: string | Buffer, options?: CopyOptions | FilterFn): Promise<void>
export function copy (src: string | Buffer, dest: string | Buffer, options: CopyOptions | FilterFn, callback: (err: Error) => void): void
export function copySync (
  src: string | Buffer,
  dest: string | Buffer,
  options?: CopyOptions | FilterFn
): void

export function emptyDir (dir: string | Buffer, callback?: (err: Error) => void): Promise<void>
export function emptyDirSync (dir: string | Buffer): void
export const emptydir: typeof emptyDir
export const emptydirSync: typeof emptyDirSync

export function createFile (file: string | Buffer, callback?: (err: Error) => void): Promise<void>
export function createFileSync (file: string | Buffer): void
export const ensureFile: typeof createFile
export const ensureFileSync: typeof createFileSync

export function createLink (srcpath: string | Buffer, dstpath: string | Buffer, callback?: (err: Error) => void): Promise<void>
export function createLinkSync (srcpath: string | Buffer, dstpath: string | Buffer): void
export const ensureLink: typeof createLink
export const ensureLinkSync: typeof createLinkSync

export function createSymlink (
  srcpath: string | Buffer,
  dstpath: string | Buffer,
  type?: 'dir' | 'file' | 'junction',
  callback?: (err: Error) => void
): Promise<void>
export function createSymlink (srcpath: string | Buffer, dstpath: string | Buffer, type?: 'dir' | 'file' | 'junction'): Promise<void>
export function createSymlink (srcpath: string | Buffer, dstpath: string | Buffer, type: 'dir' | 'file' | 'junction', callback: (err: Error) => void): void
export function createSymlinkSync (
  srcpath: string | Buffer,
  dstpath: string | Buffer,
  type?: 'dir' | 'file' | 'junction'
): void
export const ensureSymlink: typeof createSymlink
export const ensureSymlinkSync: typeof createSymlinkSync

export function readJson (file: fs.PathLike | number, options?: JsonReadOptions | BufferEncoding, callback?: (err: Error | undefined, obj: any) => void): Promise<any>
export function readJSON (file: fs.PathLike | number, options?: JsonReadOptions | BufferEncoding, callback?: (err: Error | undefined, obj: any) => void): Promise<any>
export function readJsonSync (file: fs.PathLike | number, options?: JsonReadOptions | BufferEncoding): any
export function readJSONSync (file: fs.PathLike | number, options?: JsonReadOptions | BufferEncoding): any

export function writeJson (
  file: fs.PathLike | number,
  object: any,
  callback?: (err: Error | undefined) => void
): Promise<void>
export function writeJson (
  file: fs.PathLike | number,
  object: any,
  options?: JsonWriteFileOptions | BufferEncoding | string,
  callback?: (err: Error | undefined) => void
): Promise<void>
export function writeJson (
  file: fs.PathLike | number,
  object: any,
  options?: JsonWriteFileOptions | BufferEncoding | string,
  callback?: (err: Error | undefined) => void
): void
export function writeJSON (
  file: fs.PathLike | number,
  object: any,
  callback?: (err: Error | undefined) => void
): Promise<void>
export function writeJSON (
  file: fs.PathLike | number,
  object: any,
  options?: JsonWriteFileOptions | BufferEncoding | string,
  callback?: (err: Error | undefined) => void
): Promise<void>
export function writeJsonSync (file: fs.PathLike | number, object: any, options?: JsonWriteFileOptions | BufferEncoding | string): void
export function writeJSONSync (file: fs.PathLike | number, object: any, options?: JsonWriteFileOptions | BufferEncoding | string): void

export function outputJson (file: string | Buffer, data: any, options?: JsonWriteFileOptions | BufferEncoding, callback?: (err: Error) => void): Promise<void>
export function outputJSON (file: string | Buffer, data: any, options?: JsonWriteFileOptions | BufferEncoding, callback?: (err: Error) => void): Promise<void>
export function outputJsonSync (file: string | Buffer, data: any, options?: JsonWriteFileOptions | BufferEncoding): void
export function outputJSONSync (file: string | Buffer, data: any, options?: JsonWriteFileOptions | BufferEncoding): void

export function mkdirs (dir: string | Buffer, options?: number | EnsureDirOptions, callback?: (err: Error | undefined, made?: string | undefined) => void): Promise<string | undefined>
export function mkdirsSync (dir: string | Buffer, options?: number | EnsureDirOptions): string | undefined
export const mkdirp: typeof mkdirs
export const mkdirpSync: typeof mkdirsSync
export const ensureDir: typeof mkdirs
export const ensureDirSync: typeof mkdirsSync

export function move (src: string | Buffer, dest: string | Buffer, callback?: (err: Error) => void): Promise<void>
export function move (src: string | Buffer, dest: string | Buffer, options?: MoveOptions, callback?: (err: Error) => void): Promise<void>
export function move (src: string | Buffer, dest: string | Buffer, options?: MoveOptions, callback?: (err: Error) => void): void
export function moveSync (src: string | Buffer, dest: string | Buffer, options?: MoveOptions): void

export function outputFile (
  file: fs.PathLike,
  data: string | Uint8Array,
  options?: WriteFileOptions,
  callback?: (err: Error) => void
): Promise<void>
export function outputFile (file: fs.PathLike, data: string | Uint8Array, options?: WriteFileOptions): Promise<void>
export function outputFileSync (file: fs.PathLike, data: string | Uint8Array, options?: WriteFileOptions): void

export function pathExists (path: fs.PathLike, callback?: (err: Error | undefined, exists: boolean) => void): Promise<boolean>
export function pathExistsSync (path: fs.PathLike): boolean

export function remove (path: string | Buffer, callback?: (err: Error) => void): Promise<void>
export function removeSync (path: string | Buffer): void