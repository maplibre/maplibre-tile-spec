package com.mlt.tools;

public class Timer {
    private long startTime;

    public Timer() {
        startTime = System.nanoTime();
    }

    public void restart() {
        startTime = System.nanoTime();
    }

    public void stop(String message) {
        long endTime = System.nanoTime();
        long elapsedTime = (endTime - startTime) / 1000000; // divide by 1000000 to get milliseconds
        System.out.println("Time elapsed for " + message + ": " + elapsedTime + " milliseconds");
    }
}
