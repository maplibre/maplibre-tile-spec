package org.springmeyer;

public class Point {
  public int x;
  public int y;

  public Point(int x, int y) {
    this.x = x;
    this.y = y;
  }

  public Point clone() {
    return new Point(this.x, this.y);
  }

  public String toString() {
    return "{x:" + this.x + ", y:" + this.y + "}";
  }
}
