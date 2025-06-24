package de.kybe.command.commands;

import de.kybe.command.Command;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.ToggleableModule;
import de.kybe.utils.ChatUtils;

import static de.kybe.Constants.mc;

public class ToggleCommand extends Command {
  public ToggleCommand () {
    super("toggle", "t");
  }

  @Override
  public void execute(String[] args) {
    if (args.length == 0) {;
      ChatUtils.print("Usage: .toggle <modulename>");
      return;
    }

    Module module = ModuleManager.getByNameCaseInsensitive(args[0]);
    if (module == null) {
      ChatUtils.print("Module not found: " + args[0]);
      return;
    }

    if (module instanceof ToggleableModule toggle) {
      toggle.toggle();
      ChatUtils.print((toggle.isToggled() ? "Enabled" : "Disabled") + " module: " + toggle.getName());
    } else {
      ChatUtils.print("Module " + module.getName() + " is not toggleable.");
    }
  }
}
