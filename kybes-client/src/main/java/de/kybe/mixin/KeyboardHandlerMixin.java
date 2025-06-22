package de.kybe.mixin;

import de.kybe.event.EventManager;
import de.kybe.event.events.KeyPressEvent;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.ToggleableModule;
import net.minecraft.client.KeyboardHandler;
import net.minecraft.client.gui.screens.ChatScreen;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import static de.kybe.Constants.mc;

@Mixin(KeyboardHandler.class)
public class KeyboardHandlerMixin {
  @Inject(method = "keyPress", at = @At("HEAD"))
  private void onKeyPress(long window, int key, int scancode, int action, int modifiers, CallbackInfo ci) {
    if (action != 1) return;

    KeyPressEvent event = new KeyPressEvent(key, scancode, modifiers);
    EventManager.call(event);
    if (event.isCancelled()) return;

    if (mc.screen instanceof ChatScreen || mc.screen instanceof InventoryScreen) return;

    for (Module module : ModuleManager.getAll()) {
      if (module instanceof ToggleableModule toggleable) {
        toggleable.checkToggle(key);
      }
    }
  }
}
