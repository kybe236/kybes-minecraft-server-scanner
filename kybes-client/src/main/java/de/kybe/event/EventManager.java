package de.kybe.event;

import de.kybe.event.events.Event;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;

public class EventManager {
  private static final List<Subscriber> subscribers = new ArrayList<>();

  public static void registerModule(Object module) {
    for (Method method : module.getClass().getDeclaredMethods()) {
      if (method.isAnnotationPresent(KybeEvents.class)) {
        // Validate method signature: one param extending Event
        Class<?>[] params = method.getParameterTypes();
        if (params.length == 1 && Event.class.isAssignableFrom(params[0])) {
          subscribers.add(new Subscriber(module, method));
        }
      }
    }
  }

  public static void call(Event event) {
    for (Subscriber sub : subscribers) {
      Method method = sub.method;
      if (method.getParameterTypes()[0].isAssignableFrom(event.getClass())) {
        try {
          method.setAccessible(true);
          method.invoke(sub.instance, event);
        } catch (Exception e) {
          e.printStackTrace();
        }
      }
    }
  }

  private static class Subscriber {
    final Object instance;
    final Method method;
    Subscriber(Object instance, Method method) {
      this.instance = instance;
      this.method = method;
    }
  }
}
