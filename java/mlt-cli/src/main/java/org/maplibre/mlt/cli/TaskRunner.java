package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;

///  Simplify optional parallel operation by running tasks in a thread pool if provided, or directly
// if not.
public interface TaskRunner {
  /// Get the number of threads in use, not including the main thread
  int getThreadCount();

  ///  Execute the given task either directly or on the given thread pool
  void run(@Nullable Runnable task);

  ///  Wait for all tasks to complete.  Assumes shutdown has been called.
  void awaitTermination() throws InterruptedException;

  ///  Stop accepting new tasks
  public void shutdown();
}
