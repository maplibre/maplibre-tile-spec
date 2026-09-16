package org.maplibre.mlt.cli

import me.lemire.integercompression.IntegerCODEC
import org.junit.jupiter.api.Assertions.assertNotSame
import org.junit.jupiter.api.Assertions.assertSame
import org.junit.jupiter.api.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

class FastPforCodecCacheTest {
    @Test
    fun `same thread returns the same codec instance`() {
        val cache = FastPforCodecCache()
        val first = cache.forCurrentThread()
        val second = cache.forCurrentThread()
        assertSame(first, second)
    }

    @Test
    fun `different threads receive distinct codec instances`() {
        val cache = FastPforCodecCache()
        val otherThreadCodec = AtomicReference<IntegerCODEC>()
        val started = CountDownLatch(1)
        val finished = CountDownLatch(1)

        val thread =
            Thread {
                started.countDown()
                otherThreadCodec.set(cache.forCurrentThread())
                finished.countDown()
            }
        thread.start()
        check(started.await(5, TimeUnit.SECONDS))
        val thisThreadCodec = cache.forCurrentThread()
        check(finished.await(5, TimeUnit.SECONDS))
        thread.join(5_000)

        assertNotSame(thisThreadCodec, otherThreadCodec.get())
    }
}
