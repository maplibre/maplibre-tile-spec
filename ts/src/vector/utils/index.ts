/**
 * Vector utility functions for filtering and comparison operations.
 *
 * This module provides external utility functions for working with vectors,
 * promoting composition and enabling tree-shaking.
 *
 * @module vector/utils
 */

export {
    filter,
    filterSelected,
    filterNotEqual,
    filterNotEqualSelected,
    match,
    matchSelected,
    noneMatch,
    noneMatchSelected,
    presentValues,
    presentValuesSelected,
    nullableValues,
    nullableValuesSelected,
} from './filterUtils';

export {
    greaterThanOrEqualTo,
    greaterThanOrEqualToSelected,
    smallerThanOrEqualTo,
    smallerThanOrEqualToSelected,
    type ComparableVector,
} from './comparisonUtils';
