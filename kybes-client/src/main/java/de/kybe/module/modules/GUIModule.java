package de.kybe.module.modules;

import de.kybe.module.ToggleableModule;
import de.kybe.screens.GUI;
import de.kybe.settings.ColorSetting;
import net.minecraft.client.gui.screens.ChatScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;

import static de.kybe.Constants.mc;

public class GUIModule extends ToggleableModule {
  public ColorSetting selectedColor = new ColorSetting("Selected Color", 0xFFFFFFFF);
  public ColorSetting unselectedColor = new ColorSetting("Unselected Color", 0xFF888888);
  public ColorSetting editingColor = new ColorSetting("Editing Color", 0xFFFFFF00);
  public ColorSetting cursorColor = new ColorSetting("Cursor Color", 0xFF000000);

  public GUIModule() {
    super("GUI");
    if (getToggleBind().getValue() == -1) {
      getToggleBind().setValue(61);
    }

    this.addSetting(selectedColor, unselectedColor, editingColor, cursorColor);
  }

  @Override
  protected void onToggled(boolean toggled) {
    if (!toggled) return;

    Screen screen = mc.screen;
    if (screen instanceof InventoryScreen) return;
    if (screen instanceof ChatScreen) return;
    if (screen instanceof GUI) return;

    mc.setScreen(new GUI());
    this.setToggled(false);
  }
}
