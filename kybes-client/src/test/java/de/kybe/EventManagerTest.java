package de.kybe;


import de.kybe.event.EventManager;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class EventManagerTest {

  @Test
  public void testEventCallsAllSubscribers() {
    EventManager.registerModule(new SubscriberOne());
    EventManager.registerModule(new SubscriberTwo());

    TestEvent event = new TestEvent();
    EventManager.call(event);

    // Expect count == 2 because two subscribers incremented it
    assertEquals(2, event.getCount());
  }
}