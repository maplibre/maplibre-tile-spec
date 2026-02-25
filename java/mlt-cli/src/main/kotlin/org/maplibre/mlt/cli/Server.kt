package org.maplibre.mlt.cli

import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.ServerSocket
import java.net.Socket

class Server {
    fun run(port: Int): Boolean {
        if (isRunning(port)) {
            return true
        }

        return startServer(port)
    }

    // Use `_` (unnamed variable) with JDK22+
    private fun isRunning(port: Int): Boolean {
        try {
            Socket("localhost", port).use { ignored ->
                return true
            }
        } catch (e: Exception) {
            return false
        }
    }

    private fun startServer(port: Int): Boolean {
        try {
            ServerSocket(port).use { server ->
                println("Server started on port " + port)
                while (true) {
                    val client = server.accept()
                    Thread(Runnable { handleClient(client) }).start()
                }
            }
        } catch (ex: Exception) {
            logger.error("Failed to start server on port {}", port, ex)
            return false
        }
    }

    private fun handleClient(socket: Socket) {
        try {
            BufferedReader(InputStreamReader(socket.getInputStream())).use { `in` ->
                val command = `in`.readLine()
                if (command != null) {
                    Encode.run(
                        command
                            .trim { it <= ' ' }
                            .split("\\s+".toRegex())
                            .dropLastWhile { it.isEmpty() }
                            .toTypedArray(),
                    )
                }
            }
        } catch (ex: Exception) {
            logger.error("Failed to handle client connection", ex)
        }
    }

    companion object {
        private val logger: Logger = LoggerFactory.getLogger(Server::class.java)
    }
}
