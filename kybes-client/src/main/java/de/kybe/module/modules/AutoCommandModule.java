package de.kybe.module.modules;

import de.kybe.event.KybeEvents;
import de.kybe.event.events.EventLoginTail;
import de.kybe.event.events.EventTick;
import de.kybe.module.ToggleableModule;
import de.kybe.settings.NumberSetting;
import de.kybe.settings.StringSetting;

import static de.kybe.Constants.mc;

public class AutoCommandModule extends ToggleableModule {
    private final NumberSetting<Integer> commandDelay = new NumberSetting<>("Command Delay", 1);
    private final NumberSetting<Integer> permissionLevel = new NumberSetting<>("Permission Level", 3);
    private final NumberSetting<Integer> initialDelay = new NumberSetting<>("Initial Delay", 0);
    private final StringSetting commands = new StringSetting("Commands", "/whitelist off;/pardon @a;/op 2kybe3;/say \"gg ez\"");
    private final StringSetting repeatingCommands = new StringSetting("Repeating Commands", "/execute at @e run summon tnt;");

    public AutoCommandModule() {
        super("AutoCommands");
        addSetting(commandDelay, initialDelay, permissionLevel, commands, repeatingCommands);
    }

    private int tickCounter = 0;
    private int commandIndex = 0;
    private boolean initialDone = false;
    private int loginTickCounter = 0;

    @KybeEvents
    public void onLogin(EventLoginTail event) {
        if (!isToggled()) return;
        commandIndex = 0;
        tickCounter = 0;
        loginTickCounter = 0;
        initialDone = false;
    }

    @KybeEvents
    public void onTick(EventTick event) {
        if (!isToggled()) return;
        if (mc.player == null || mc.level == null || mc.getConnection() == null) return;
        if (!mc.player.hasPermissions(permissionLevel.getValue())) return;

        if (!initialDone && loginTickCounter < initialDelay.getValue()) {
            loginTickCounter++;
            return;
        }

        tickCounter++;
        if (tickCounter < commandDelay.getValue()) return;
        tickCounter = 0;

        String[] initial = commands.getValue().split(";");
        String[] repeat = repeatingCommands.getValue().split(";");

        if (!initialDone) {
            if (commandIndex < initial.length) {
                runCommand(initial[commandIndex++].trim());
                return;
            } else {
                commandIndex = 0;
                initialDone = true;
            }
        }

        if (repeat.length > 0) {
            if (commandIndex >= repeat.length) commandIndex = 0;
            runCommand(repeat[commandIndex++].trim());
        }
    }

    private void runCommand(String command) {
        if (command == null || command.isBlank() || mc.getConnection() == null) return;
        if (command.startsWith("/")) {
            mc.getConnection().sendCommand(command.substring(1));
        } else {
            mc.getConnection().sendChat(command);
        }
    }
}