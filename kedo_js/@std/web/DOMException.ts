class DOMException extends Error {
    // Define the readonly attributes for the name and message
    readonly name: string;
    readonly message: string;
    readonly code: number;

    // Define the error codes as constants
    static readonly INDEX_SIZE_ERR = 1;
    static readonly DOMSTRING_SIZE_ERR = 2;
    static readonly HIERARCHY_REQUEST_ERR = 3;
    static readonly WRONG_DOCUMENT_ERR = 4;
    static readonly INVALID_CHARACTER_ERR = 5;
    static readonly NO_DATA_ALLOWED_ERR = 6;
    static readonly NO_MODIFICATION_ALLOWED_ERR = 7;
    static readonly NOT_FOUND_ERR = 8;
    static readonly NOT_SUPPORTED_ERR = 9;
    static readonly INUSE_ATTRIBUTE_ERR = 10;
    static readonly INVALID_STATE_ERR = 11;
    static readonly SYNTAX_ERR = 12;
    static readonly INVALID_MODIFICATION_ERR = 13;
    static readonly NAMESPACE_ERR = 14;
    static readonly INVALID_ACCESS_ERR = 15;
    static readonly VALIDATION_ERR = 16;
    static readonly TYPE_MISMATCH_ERR = 17;
    static readonly SECURITY_ERR = 18;
    static readonly NETWORK_ERR = 19;
    static readonly ABORT_ERR = 20;
    static readonly URL_MISMATCH_ERR = 21;
    static readonly QUOTA_EXCEEDED_ERR = 22;
    static readonly TIMEOUT_ERR = 23;
    static readonly INVALID_NODE_TYPE_ERR = 24;
    static readonly DATA_CLONE_ERR = 25;

    // Map to associate error names with codes
    private static readonly errorCodes: { [key: string]: number } = {
        IndexSizeError: DOMException.INDEX_SIZE_ERR,
        DOMStringSizeError: DOMException.DOMSTRING_SIZE_ERR,
        HierarchyRequestError: DOMException.HIERARCHY_REQUEST_ERR,
        WrongDocumentError: DOMException.WRONG_DOCUMENT_ERR,
        InvalidCharacterError: DOMException.INVALID_CHARACTER_ERR,
        NoDataAllowedError: DOMException.NO_DATA_ALLOWED_ERR,
        NoModificationAllowedError: DOMException.NO_MODIFICATION_ALLOWED_ERR,
        NotFoundError: DOMException.NOT_FOUND_ERR,
        NotSupportedError: DOMException.NOT_SUPPORTED_ERR,
        InUseAttributeError: DOMException.INUSE_ATTRIBUTE_ERR,
        InvalidStateError: DOMException.INVALID_STATE_ERR,
        SyntaxError: DOMException.SYNTAX_ERR,
        InvalidModificationError: DOMException.INVALID_MODIFICATION_ERR,
        NamespaceError: DOMException.NAMESPACE_ERR,
        InvalidAccessError: DOMException.INVALID_ACCESS_ERR,
        ValidationError: DOMException.VALIDATION_ERR,
        TypeMismatchError: DOMException.TYPE_MISMATCH_ERR,
        SecurityError: DOMException.SECURITY_ERR,
        NetworkError: DOMException.NETWORK_ERR,
        AbortError: DOMException.ABORT_ERR,
        URLMismatchError: DOMException.URL_MISMATCH_ERR,
        QuotaExceededError: DOMException.QUOTA_EXCEEDED_ERR,
        TimeoutError: DOMException.TIMEOUT_ERR,
        InvalidNodeTypeError: DOMException.INVALID_NODE_TYPE_ERR,
        DataCloneError: DOMException.DATA_CLONE_ERR,
    };

    constructor(message: string = "", name: string = "Error") {
        super(message); // Call the Error constructor

        this.name = name;
        this.message = message;
        this.code = DOMException.errorCodes[name] || 0;

        // Set the prototype explicitly to maintain the correct instance type
        // Object.setPrototypeOf(this, DOMException.prototype);
        Error.captureStackTrace(this, this.constructor);
    }

    // Serialization method to convert the exception into a JSON string
    toJSON(): { name: string; message: string; code: number } {
        return {
            name: this.name,
            message: this.message,
            code: this.code,
        };
    }

    // Static method for deserializing a JSON object back to a DOMException
    static fromJSON(serialized: {
        name: string;
        message: string;
    }): DOMException {
        return new DOMException(serialized.message, serialized.name);
    }
}

export { DOMException };
