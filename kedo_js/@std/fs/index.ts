import {
  op_fs_read_dir,
  op_fs_read_dir_sync,
  op_fs_read_file,
  op_fs_read_file_sync,
  op_fs_remove,
  op_fs_remove_sync,
  op_fs_write_file,
  op_fs_write_file_sync,
} from "@kedo/internal/utils";
import { asyncOp } from "@kedo/utils";

export class DirEntry {
  readonly name: string;
  readonly isFile: boolean;
  readonly isDir: boolean;
  readonly isSymlink: boolean;
  readonly parentPath: string;

  constructor(
    name: string,
    isFile: boolean,
    isDir: boolean,
    isSymlink: boolean,
    parentPath: string,
  ) {
    this.name = name;
    this.isFile = isFile;
    this.isDir = isDir;
    this.isSymlink = isSymlink;
    this.parentPath = parentPath;
  }
}

function readFileSync(path: string): string {
  return op_fs_read_file_sync(path);
}

function readDirSync(path: string): DirEntry[] {
  return op_fs_read_dir_sync(path);
}

function writeFileSync(path: string, data: string): void {
  return op_fs_write_file_sync(path, data);
}

function removeSync(path: string, recursive: boolean): void {
  return op_fs_remove_sync(path, recursive);
}

function readFile(path: string): Promise<string> {
  return asyncOp(op_fs_read_file, path);
}

function writeFile(path: string, data: string): Promise<void> {
  return asyncOp(op_fs_write_file, path, data);
}

function readDir(path: string): Promise<DirEntry> {
  return asyncOp(op_fs_read_dir, path);
}

function remove(path: string, recursive: boolean = false): Promise<void> {
  return asyncOp(op_fs_remove, path, recursive);
}

Kedo.readFileSync = readFileSync;
Kedo.readDirSync = readDirSync;
Kedo.writeFileSync = writeFileSync;
Kedo.removeSync = removeSync;
Kedo.readFile = readFile;
Kedo.writeFile = writeFile;
Kedo.readDir = readDir;
Kedo.remove = remove;
