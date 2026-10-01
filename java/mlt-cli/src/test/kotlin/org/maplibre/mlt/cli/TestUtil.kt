package org.maplibre.mlt.cli

import org.junit.jupiter.api.Assertions.assertTrue

fun Exception.andContains(str: String) =
    assertTrue(this.message?.contains(str) ?: false, "Expected exception message to contain '$str', but was '${this.message}'")
