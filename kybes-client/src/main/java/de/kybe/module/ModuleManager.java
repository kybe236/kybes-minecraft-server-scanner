package de.kybe.module;

import de.kybe.event.EventManager;

import java.util.Collection;
import java.util.HashMap;
import java.util.Map;

public class ModuleManager {
  private static final Map<String, Module> modules = new HashMap<>();

  public static void register(Module module) {
    modules.put(module.getName(), module);
    EventManager.registerModule(module);
  }

  public static void clearModules() {
    modules.clear();
  }

  public static Module getByName(String name) {
    return modules.get(name);
  }

  public static Module getByNameCaseInsensitive(String name) {
    return modules.values().stream()
        .filter(module -> module.getName().equalsIgnoreCase(name))
        .findFirst()
        .orElse(null);
  }

  public static Collection<Module> getAll() {
    return modules.values();
  }
}
