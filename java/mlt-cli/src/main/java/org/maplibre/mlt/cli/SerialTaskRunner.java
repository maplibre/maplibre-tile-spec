package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;

public class SerialTaskRunner implements TaskRunner {
  public SerialTaskRunner() {}

  public int getThreadCount() {
    return 0;
  }

  public void run(@Nullable Runnable task) {
    if (task != null) {
      task.run();
    }
  }

  public void awaitTermination() throws InterruptedException {}

  public void shutdown() {}
}
