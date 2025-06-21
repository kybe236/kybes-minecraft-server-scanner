package de.kybe.mixin;

import net.minecraft.SharedConstants;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(SharedConstants.class)
public class SharedConstantsMixin {
  @Inject(method = "<clinit>", at = @At("RETURN"))
  private static void onClassInit(CallbackInfo ci) {
    SharedConstants.IS_RUNNING_IN_IDE = false;
  }
}