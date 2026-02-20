package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import org.jetbrains.annotations.NotNull;

public class ThreadPoolTaskRunner implements TaskRunner {
  public ThreadPoolTaskRunner(@NotNull ThreadPoolExecutor threadPool) {
    this.threadPool = threadPool;
  }

  @Override
  public int getThreadCount() {
    return threadPool.getMaximumPoolSize();
  }

  @Override
  public void run(@Nullable Runnable task) {
    if (task != null) {
      threadPool.execute(task);
    }
  }

  @Override
  public void awaitTermination() throws InterruptedException {
    threadPool.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS);
  }

  @Override
  public void shutdown() {
    threadPool.shutdown();
  }

  private final @NotNull ThreadPoolExecutor threadPool;
}
