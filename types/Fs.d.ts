declare module "@kedo/fs" {
    /**
     * Represents a directory entry and provides information about its type and parent location.
     *
     * @property name - The name of the directory entry.
     * @property isFile - Indicates whether the entry is a file.
     * @property isDir - Indicates whether the entry is a directory.
     * @property isSymlink - Indicates whether the entry is a symbolic link.
     * @property parentPath - The path of the parent directory containing this entry.
     */
    export class DirEntry {
        readonly name: string;
        readonly isFile: boolean;
        readonly isDir: boolean;
        readonly isSymlink: boolean;
        readonly parentPath: string;
    }
}