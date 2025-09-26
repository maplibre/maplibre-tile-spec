package org.maplibre.mlt.cli;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.ServerSocket;
import java.net.Socket;

public class Server {

  public boolean run(int port) {
    if (isRunning(port)) {
      return true;
    }

    return startServer(port);
  }

  private boolean isRunning(int port) {
    try (Socket client = new Socket("localhost", port)) {
      return true;
    } catch (Exception e) {
      return false;
    }
  }

  private boolean startServer(int port) {
    try (ServerSocket server = new ServerSocket(port)) {
      System.out.println("Server started on port " + port);

      while (true) {
        Socket client = server.accept();

        new Thread(() -> handleClient(client)).start();
      }
    } catch(Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
      return false;
    }
  }

  private void handleClient(Socket socket) {
    try (
      BufferedReader in = new BufferedReader(new InputStreamReader(socket.getInputStream()));
      PrintWriter out = new PrintWriter(socket.getOutputStream(), true);
    ) {
      try {
        String command = in.readLine();

        if (command != null) {
          Encode.run(command.trim().split("\\s+"));
        }
      } catch(Exception e) {
        e.printStackTrace(out);
      }
    } catch(Exception e) {
      System.err.println("Failed:");
      e.printStackTrace(System.err);
    }
  }
}
