package de.kybe;

import de.kybe.event.KybeEvents;

public class SubscriberOne {
  @KybeEvents
  public void onTestEvent(TestEvent event) {
    event.increment();
  }
}