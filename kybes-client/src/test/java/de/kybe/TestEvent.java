package de.kybe;

import de.kybe.event.events.Event;

public class TestEvent extends Event {
  private int count = 0;

  public int getCount() {
    return count;
  }

  public void increment() {
    count++;
  }
}
