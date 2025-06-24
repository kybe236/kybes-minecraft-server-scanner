package de.kybe.command;

import java.util.Arrays;
import java.util.List;

public abstract class Command {
  private final String name;
  private final List<String> aliases;

  public Command(String name, String... aliases) {
    this.name = name;
    this.aliases = Arrays.asList(aliases);
  }

  public String getName() {
    return name;
  }

  public List<String> getAliases() {
    return aliases;
  }

  public abstract void execute(String[] args);
}
