package de.kybe.config;

import com.google.gson.*;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.ToggleableModule;
import de.kybe.settings.Setting;
import net.minecraft.client.Minecraft;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;

public class Config {
  private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

  public static void save() {
    save(Minecraft.getInstance().gameDirectory.toPath().resolve("kybe.conf"));
  }

  public static void load() {
    load(Minecraft.getInstance().gameDirectory.toPath().resolve("kybe.conf"));
  }

  public static void save(Path path) {
    JsonArray moduleArray = new JsonArray();

    for (Module module : ModuleManager.getAll()) {
      JsonObject moduleJson = new JsonObject();
      moduleJson.addProperty("name", module.getName());

      if (module instanceof ToggleableModule toggleable) {
        moduleJson.addProperty("toggled", toggleable.isToggled());
      }

      JsonArray settingsArray = new JsonArray();
      for (Setting<?> setting : module.getSettings()) {
        settingsArray.add(setting.toJson());
      }

      moduleJson.add("settings", settingsArray);
      moduleArray.add(moduleJson);
    }

    try (Writer writer = Files.newBufferedWriter(path)) {
      GSON.toJson(moduleArray, writer);
    } catch (IOException e) {
      e.printStackTrace();
    }
  }

  public static void load(Path path) {
    if (!Files.exists(path)) return;

    try (Reader reader = Files.newBufferedReader(path)) {
      JsonArray moduleArray = JsonParser.parseReader(reader).getAsJsonArray();

      for (JsonElement el : moduleArray) {
        JsonObject moduleJson = el.getAsJsonObject();
        String name = moduleJson.get("name").getAsString();

        Module module = ModuleManager.getByName(name);
        if (module == null) continue;

        if (module instanceof ToggleableModule toggleable && moduleJson.has("toggled")) {
          toggleable.setToggled(moduleJson.get("toggled").getAsBoolean());
        }

        if (moduleJson.has("settings")) {
          JsonArray settingsArray = moduleJson.getAsJsonArray("settings");
          for (JsonElement s : settingsArray) {
            JsonObject sJson = s.getAsJsonObject();
            String sName = sJson.get("name").getAsString();

            for (Setting<?> setting : module.getSettings()) {
              if (setting.getName().equals(sName)) {
                setting.fromJson(sJson);
                break;
              }
            }
          }
        }
      }
    } catch (IOException e) {
      e.printStackTrace();
    }
  }
}
