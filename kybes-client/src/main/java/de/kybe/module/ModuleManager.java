package de.kybe.module;

import java.util.Collection;
import java.util.HashMap;
import java.util.Map;

public class ModuleManager {
  private static final Map<String, Module> modules = new HashMap<>();

  public static void register(Module module) {
    modules.put(module.getName(), module);
  }

  public static void clearModules() {
    modules.clear();
  }

  public static Module getByName(String name) {
    return modules.get(name);
  }

  public static Collection<Module> getAll() {
    return modules.values();
  }
}
