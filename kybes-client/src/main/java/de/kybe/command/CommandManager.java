package de.kybe.command;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public class CommandManager {
  private static final List<Command> commands = new ArrayList<>();

  public static void register(Command command) {
    commands.add(command);
  }

  public static void handleInput(String input) {
    if (!input.startsWith("+")) return;

    String[] parts = input.substring(1).split(" ");
    String name = parts[0];
    String[] args = Arrays.copyOfRange(parts, 1, parts.length);

    for (Command command : commands) {
      if (command.getName().equalsIgnoreCase(name) ||
        command.getAliases().stream().anyMatch(alias -> alias.equalsIgnoreCase(name))) {
        command.execute(args);
        return;
      }
    }
  }
}
