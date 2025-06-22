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
        Class<?>[] params = method.getParameterTypes();
        if (params.length == 1 && Event.class.isAssignableFrom(params[0])) {
          subscribers.add(new Subscriber(module, method));
        }
      }
    }
  }

  @SuppressWarnings("CallToPrintStackTrace")
  public static void call(Event event) {
    for (Subscriber sub : subscribers) {
      Method method = sub.method;
      if (method.getParameterTypes()[0].isAssignableFrom(event.getClass())) {
        try {
          method.setAccessible(true);
          method.invoke(sub.instance, event);
        } catch (Exception e) {
          e.printStackTrace();
          System.err.println("Failed to invoke event handler: " + method.getName() + " in " + sub.instance.getClass().getName());
        }
      }
    }
  }

  private record Subscriber(Object instance, Method method) {
  }
}
