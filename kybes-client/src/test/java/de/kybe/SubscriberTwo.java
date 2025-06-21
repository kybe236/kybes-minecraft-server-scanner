package de.kybe;

import de.kybe.event.KybeEvents;

public class SubscriberTwo {
  @KybeEvents
  public void onTestEvent(TestEvent event) {
    event.increment();
  }
}