package de.kybe.screens;

import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.ToggleableModule;
import de.kybe.module.modules.GUIModule;
import de.kybe.settings.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.lwjgl.glfw.GLFW;

import java.util.ArrayList;
import java.util.List;

public class GUI extends Screen {
  private final List<Module> modules = ModuleManager.getAll().stream().toList();
  private final ArrayList<Integer> settingIndexes = new ArrayList<>();
  private int moduleIndex = 0;
  private int subSettingDepth = 0;
  private boolean inSettings = false;
  private boolean editing = false;
  private String editBuffer = "";
  private boolean waitingForBindKey = false;
  private int cursorPosition = 0;
  private boolean allSelected = false;
  public GUI() {
    super(Component.literal("KYBE"));
    settingIndexes.add(0);
  }

  private int getSelectedColor() {
    GUIModule guiModule = (GUIModule) ModuleManager.getByName("GUI");
    return guiModule != null ? guiModule.selectedColor.getValue() : 0xFFFFFFFF;
  }

  private int getUnselectedColor() {
    GUIModule guiModule = (GUIModule) ModuleManager.getByName("GUI");
    return guiModule != null ? guiModule.unselectedColor.getValue() : 0xFF888888;
  }

  private int getEditingColor() {
    GUIModule guiModule = (GUIModule) ModuleManager.getByName("GUI");
    return guiModule != null ? guiModule.editingColor.getValue() : 0xFFFFFF00;
  }

  private int getCursorColor() {
    GUIModule guiModule = (GUIModule) ModuleManager.getByName("GUI");
    return guiModule != null ? guiModule.cursorColor.getValue() : 0xFF00FF00;
  }

  @Override
  public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
    super.render(graphics, mouseX, mouseY, delta);
    Font font = Minecraft.getInstance().font;

    int yPos = 20;
    for (int i = 0; i < modules.size(); i++) {
      Module module = modules.get(i);
      String state = (module instanceof ToggleableModule t && t.isToggled()) ? "[ON] " : "[OFF] ";
      int color = (i == moduleIndex) ? getSelectedColor() : getUnselectedColor();
      graphics.drawString(font, state + module.getName(), 10, yPos, color);
      yPos += 12;
    }

    if (inSettings) {
      yPos = 20;
      List<Setting<?>> currentSettings = getSelectedSettings();
      int currentIndex = settingIndexes.get(subSettingDepth);

      for (int i = 0; i < currentSettings.size(); i++) {
        Setting<?> setting = currentSettings.get(i);
        String text;
        int color;
        if (setting instanceof ColorSetting colorSetting) {
          text = setting.getName() + ": " + String.format("#%08X", colorSetting.getValue());
          if (i == currentIndex) {
            color = blendColors(colorSetting.getValue(), 0xFFFFFFFF, 0.7f);
          } else {
            color = colorSetting.getValue();
          }
        } else {
          text = setting.getName() + ": " + setting.getValue();
          color = (i == currentIndex) ? getSelectedColor() : getUnselectedColor();
        }
        graphics.drawString(font, text, 200 + subSettingDepth * 20, yPos, color);

        graphics.drawString(font, text, 200 + subSettingDepth * 20, yPos, color);
        yPos += 12;
      }

      if (editing) {
        String displayText = editBuffer;
        int baseX = 200 + subSettingDepth * 20;

        int prefixWidth = font.width("> ");

        int cursorX = baseX + prefixWidth + font.width(displayText.substring(0, cursorPosition));

        if (allSelected) {
          graphics.fill(
            baseX + prefixWidth,
            yPos + 9,
            baseX + prefixWidth + font.width(displayText),
            yPos + 9 + font.lineHeight,
            0x77FFFFFF
          );
        }

        graphics.drawString(font, "> " + displayText + " <", baseX, yPos + 10, getEditingColor());

        graphics.fill(cursorX, yPos + 10, cursorX + 1, yPos + 10 + font.lineHeight, getCursorColor());
      }


      if (waitingForBindKey) {
        graphics.drawString(font, "Press a key to bind...", 200 + subSettingDepth * 20, yPos + 10, getEditingColor());
      }
    }

    List<String> lines = new ArrayList<>();
    lines.add("Made by 2kybe3 / kybe236");
    lines.add("Current Module: " + getSelectedModule().getName());
    lines.add("Current Setting: " + getCurrentSetting().getName());
    lines.add("Current Module Index: " + moduleIndex);
    lines.add("Nesting Level: " + subSettingDepth);

    drawStrings(font, graphics, width, height, lines, 2, getUnselectedColor());
  }

  private int blendColors(int color1, int color2, float ratio) {
    float r1 = ((color1 >> 16) & 0xFF) / 255f;
    float g1 = ((color1 >> 8) & 0xFF) / 255f;
    float b1 = (color1 & 0xFF) / 255f;
    float a1 = ((color1 >> 24) & 0xFF) / 255f;

    float r2 = ((color2 >> 16) & 0xFF) / 255f;
    float g2 = ((color2 >> 8) & 0xFF) / 255f;
    float b2 = (color2 & 0xFF) / 255f;
    float a2 = ((color2 >> 24) & 0xFF) / 255f;

    float r = r1 * (1 - ratio) + r2 * ratio;
    float g = g1 * (1 - ratio) + g2 * ratio;
    float b = b1 * (1 - ratio) + b2 * ratio;
    float a = a1 * (1 - ratio) + a2 * ratio;

    int ir = (int) (r * 255) & 0xFF;
    int ig = (int) (g * 255) & 0xFF;
    int ib = (int) (b * 255) & 0xFF;
    int ia = (int) (a * 255) & 0xFF;

    return (ia << 24) | (ir << 16) | (ig << 8) | ib;
  }


  public void drawStrings(Font font, GuiGraphics graphics, int width, int height, List<String> lines, int lineSpacing, int color) {
    int y = height - 15;
    for (int i = lines.size() - 1; i >= 0; i--) {
      String line = lines.get(i);
      int textWidth = font.width(line);
      y -= font.lineHeight + lineSpacing;
      graphics.drawString(font, line, width - textWidth - 5, y, color);
    }
  }

  @Override
  public boolean keyPressed(int keyCode, int scanCode, int modifiers) {
    if (editing) {
      // Handle Ctrl+A (select all)
      if ((modifiers & GLFW.GLFW_MOD_CONTROL) != 0 && keyCode == GLFW.GLFW_KEY_A) {
        allSelected = true;
        return true;
      }

      // Handle Ctrl+C (copy)
      if ((modifiers & GLFW.GLFW_MOD_CONTROL) != 0 && keyCode == GLFW.GLFW_KEY_C) {
        Minecraft.getInstance().keyboardHandler.setClipboard(editBuffer);
        return true;
      }

      // Handle arrow keys for cursor navigation
      if (keyCode == GLFW.GLFW_KEY_LEFT) {
        cursorPosition = Math.max(0, cursorPosition - 1);
        allSelected = false;
        return true;
      } else if (keyCode == GLFW.GLFW_KEY_RIGHT) {
        cursorPosition = Math.min(editBuffer.length(), cursorPosition + 1);
        allSelected = false;
        return true;
      }

      // Handle Del with Ctrl+A
      if (allSelected && keyCode == GLFW.GLFW_KEY_DELETE) {
        editBuffer = "";
        cursorPosition = 0;
        allSelected = false;
        return true;
      }

      // Handle regular Del key
      if (keyCode == GLFW.GLFW_KEY_DELETE && cursorPosition < editBuffer.length()) {
        if (allSelected) {
          editBuffer = "";
          cursorPosition = 0;
          allSelected = false;
        } else {
          editBuffer = editBuffer.substring(0, cursorPosition) +
            editBuffer.substring(cursorPosition + 1);
        }
        return true;
      }
    }

    if (editing && (modifiers & GLFW.GLFW_MOD_CONTROL) != 0 && keyCode == GLFW.GLFW_KEY_V) {
      try {
        String clipText = Minecraft.getInstance().keyboardHandler.getClipboard();
        if (!clipText.isEmpty()) {
          if (allSelected) {
            editBuffer = "";
            cursorPosition = 0;
            allSelected = false;
          }
          editBuffer = editBuffer.substring(0, cursorPosition) + clipText +
            editBuffer.substring(cursorPosition);
          cursorPosition += clipText.length();
        }
      } catch (Exception e) {
        e.printStackTrace();
      }
      return true;
    }

    if (waitingForBindKey) {
      if (keyCode == GLFW.GLFW_KEY_ESCAPE) {
        waitingForBindKey = false;
        return true;
      }
      if (keyCode == GLFW.GLFW_KEY_DELETE) {
        Setting<?> setting = getCurrentSetting();
        if (setting instanceof BindSetting bindSetting) {
          bindSetting.setValue(-1);
        }
        waitingForBindKey = false;
        return true;
      }
      if (keyCode != GLFW.GLFW_KEY_ENTER) {
        Setting<?> setting = getCurrentSetting();
        if (setting instanceof BindSetting bindSetting) {
          bindSetting.setValue(keyCode);
        }
        waitingForBindKey = false;
        return true;
      }
      return true;
    }

    if (!editing && handleNavigationKeys(keyCode)) return true;
    if (handleEnterKey(keyCode)) return true;
    if (handleEditKeys(keyCode)) return true;

    return super.keyPressed(keyCode, scanCode, modifiers);
  }

  private boolean handleNavigationKeys(int keyCode) {
    switch (keyCode) {
      case GLFW.GLFW_KEY_UP, GLFW.GLFW_KEY_W -> moveUp();    // UP/W
      case GLFW.GLFW_KEY_DOWN, GLFW.GLFW_KEY_S -> moveDown();  // DOWN/S
      case GLFW.GLFW_KEY_RIGHT, GLFW.GLFW_KEY_D -> moveRight(); // RIGHT/D
      case GLFW.GLFW_KEY_LEFT, GLFW.GLFW_KEY_A -> moveLeft();  // LEFT/A
      default -> {
        return false;
      }
    }
    return true;
  }

  private void moveUp() {
    if (inSettings) {
      int currentIndex = getCurrentSettingIndex();
      if (currentIndex > 0) setCurrentSettingIndex(currentIndex - 1);
    } else if (moduleIndex > 0) {
      moduleIndex--;
    }
  }

  private void moveDown() {
    if (inSettings) {
      List<Setting<?>> settings = getSelectedSettings();
      int currentIndex = getCurrentSettingIndex();
      if (currentIndex < settings.size() - 1) setCurrentSettingIndex(currentIndex + 1);
    } else if (moduleIndex < modules.size() - 1) {
      moduleIndex++;
    }
  }

  private void moveRight() {
    if (!inSettings) {
      if (!getSelectedModule().getSettings().isEmpty()) {
        enterSettings();
      }
    } else {
      Setting<?> current = getCurrentSetting();
      if (!current.getSubSettings().isEmpty()) {
        enterSubSettings();
      }
    }
  }

  private void enterSettings() {
    inSettings = true;
    subSettingDepth = 0;
    settingIndexes.clear();
    settingIndexes.add(0);
  }

  private void enterSubSettings() {
    subSettingDepth++;
    if (settingIndexes.size() <= subSettingDepth) {
      settingIndexes.add(0);
    }
  }

  private void moveLeft() {
    if (inSettings) {
      if (subSettingDepth > 0) {
        subSettingDepth--;
      } else {
        inSettings = false;
      }
    }
  }

  private boolean handleEnterKey(int keyCode) {
    if (keyCode != GLFW.GLFW_KEY_ENTER) return false; // ENTER

    if (!inSettings) {
      Module module = getSelectedModule();
      if (module instanceof ToggleableModule toggle) {
        toggle.setToggled(!toggle.isToggled());
      }
    } else if (!editing) {
      startEditing();
    } else {
      confirmEdit();
    }
    return true;
  }

  private void startEditing() {
    Setting<?> setting = getCurrentSetting();
    cursorPosition = 0;
    allSelected = false;

    if (setting instanceof BindSetting) {
      waitingForBindKey = true;
      editBuffer = "";
    } else if (setting instanceof BooleanSetting bool) {
      bool.setValue(!bool.getValue());
    } else if (setting instanceof NumberSetting || setting instanceof StringSetting) {
      editing = true;
      editBuffer = setting.getValue().toString();
      cursorPosition = editBuffer.length();
    } else if (setting instanceof ColorSetting colorSetting) {
      editing = true;
      // Show color as hex string, e.g. "FF00FF00"
      editBuffer = String.format("%08X", colorSetting.getValue());
      cursorPosition = editBuffer.length();
    }
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  private void confirmEdit() {
    Setting<?> setting = getCurrentSetting();
    try {
      if (setting instanceof NumberSetting num) {
        double val = Double.parseDouble(editBuffer);
        if (num.getValue() instanceof Integer) ((NumberSetting<Integer>) num).setValue((int) val);
        else if (num.getValue() instanceof Double) ((NumberSetting<Double>) num).setValue(val);
        else if (num.getValue() instanceof Float) ((NumberSetting<Float>) num).setValue((float) val);
        else if (num.getValue() instanceof Long) ((NumberSetting<Long>) num).setValue((long) val);
      } else if (setting instanceof StringSetting str) {
        str.setValue(editBuffer);
      } else if (setting instanceof ColorSetting colorSetting) {
        // If input is #RRGGBB, add full alpha #FFRRGGBB
        String input = editBuffer.trim();
        if (input.matches("^#([0-9a-fA-F]{6})$")) {
          input = "#FF" + input.substring(1);
        }
        colorSetting.setValue(input);
      }
      editing = false;
      allSelected = false;
    } catch (NumberFormatException ignored) {
      // Invalid number input
    }
  }

  private boolean handleEditKeys(int keyCode) {
    if (editing) {
      if (keyCode == GLFW.GLFW_KEY_BACKSPACE) {
        if (allSelected) {
          editBuffer = "";
          cursorPosition = 0;
          allSelected = false;
        } else if (!editBuffer.isEmpty() && cursorPosition > 0) {
          editBuffer = editBuffer.substring(0, cursorPosition - 1) +
            editBuffer.substring(cursorPosition);
          cursorPosition--;
        }
        return true;
      } else if (keyCode == GLFW.GLFW_KEY_ESCAPE) {
        editing = false;
        allSelected = false;
        return true;
      }
    }
    return false;
  }

  @Override
  public boolean charTyped(char c, int keyCode) {
    if (editing && Character.isDefined(c) && !Character.isISOControl(c)) {
      if (allSelected) {
        editBuffer = "";
        cursorPosition = 0;
        allSelected = false;
      }
      editBuffer = editBuffer.substring(0, cursorPosition) + c +
        editBuffer.substring(cursorPosition);
      cursorPosition++;
      return true;
    }
    return super.charTyped(c, keyCode);
  }

  // Helper methods
  private Module getSelectedModule() {
    return modules.get(moduleIndex);
  }

  private int getCurrentSettingIndex() {
    ensureSettingIndexCapacity();
    return settingIndexes.get(subSettingDepth);
  }

  private void setCurrentSettingIndex(int index) {
    ensureSettingIndexCapacity();
    settingIndexes.set(subSettingDepth, index);
  }

  private void ensureSettingIndexCapacity() {
    while (settingIndexes.size() <= subSettingDepth) {
      settingIndexes.add(0);
    }
  }

  private List<Setting<?>> getSelectedSettings() {
    List<Setting<?>> settings = getSelectedModule().getSettings();
    for (int i = 0; i < subSettingDepth && !settings.isEmpty(); i++) {
      int idx = (i < settingIndexes.size()) ? settingIndexes.get(i) : 0;
      if (idx >= 0 && idx < settings.size()) {
        settings = settings.get(idx).getSubSettings();
      }
    }
    return settings;
  }

  private Setting<?> getCurrentSetting() {
    List<Setting<?>> settings = getSelectedModule().getSettings();
    Setting<?> current = null;
    for (int i = 0; i <= subSettingDepth && !settings.isEmpty(); i++) {
      int idx = (i < settingIndexes.size()) ? settingIndexes.get(i) : 0;
      if (idx < 0 || idx >= settings.size()) idx = 0;
      current = settings.get(idx);
      settings = current.getSubSettings();
    }
    return current;
  }

  @Override
  public boolean isPauseScreen() {
    return false;
  }
}