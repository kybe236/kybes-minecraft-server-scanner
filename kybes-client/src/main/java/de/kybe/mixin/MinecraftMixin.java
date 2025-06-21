package de.kybe.mixin;

import de.kybe.config.Config;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Minecraft.class)
public class MinecraftMixin {

  @Unique
  private int tickCounter = 0;
  @Unique
  private static final int TICKS_PER_MINUTE = 20 * 60;

  @Inject(method = "runTick", at = @At("HEAD"))
  private void runTick(boolean bl, CallbackInfo ci) {
    tickCounter++;
    if (tickCounter >= TICKS_PER_MINUTE) {
      tickCounter = 0;
      Config.save();
    }
  }
}