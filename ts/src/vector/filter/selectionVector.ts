

export interface SelectionVector{
    getIndex(index: number): number;
    setIndex(index: number, value: number): void;
    setLimit(limit: number): void;
    selectionValues(): number[];
    /* Index of the first element that should not be read or written.
     * It's not the last index that can be accessed, but rather the index that marks the end of
     * the valid data in the buffer */
    get limit();
    /* Total size of the buffer */
    get capacity();
}
