package de.kybe.mixin;

import de.kybe.event.EventManager;
import de.kybe.event.events.EventTick;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Minecraft.class)
public class MinecraftMixin {
  @Inject(method = "runTick", at = @At("HEAD"))
  private void runTick(boolean bl, CallbackInfo ci) {
    EventTick event = new EventTick();
    EventManager.call(event);
  }
}