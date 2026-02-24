package org.maplibre.mlt.cli;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.ServerSocket;
import java.net.Socket;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class Server {

  public boolean run(int port) {
    if (isRunning(port)) {
      return true;
    }

    return startServer(port);
  }

  @SuppressWarnings("try") // Use `_` (unnamed variable) with JDK22+
  private boolean isRunning(int port) {
    try (final Socket ignored = new Socket("localhost", port)) {
      return true;
    } catch (Exception e) {
      return false;
    }
  }

  @SuppressWarnings("InfiniteLoopStatement")
  private boolean startServer(int port) {
    try (ServerSocket server = new ServerSocket(port)) {
      System.out.println("Server started on port " + port);

      while (true) {
        final var client = server.accept();
        new Thread(() -> handleClient(client)).start();
      }
    } catch (Exception ex) {
      Logger.error("Failed to start server on port {}", port, ex);
      return false;
    }
  }

  private void handleClient(Socket socket) {
    try (BufferedReader in = new BufferedReader(new InputStreamReader(socket.getInputStream()))) {
      final var command = in.readLine();
      if (command != null) {
        Encode.run(command.trim().split("\\s+"));
      }
    } catch (Exception ex) {
      Logger.error("Failed to handle client connection", ex);
    }
  }

  private static Logger Logger = LoggerFactory.getLogger(Server.class);
}
