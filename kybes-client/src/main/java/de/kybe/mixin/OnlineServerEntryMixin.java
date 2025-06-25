package de.kybe.mixin;

import de.kybe.screens.ServerPlayerListScreen;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.multiplayer.ServerSelectionList;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

import static de.kybe.Constants.mc;

@Mixin(ServerSelectionList.OnlineServerEntry.class)
public class OnlineServerEntryMixin {
  @Unique
  private final int buttonWidth = 15;
  @Unique
  private final int buttonHeight = 15;
  @Unique
  private int buttonX;
  @Unique
  private int buttonY;

  @Inject(method = "render", at = @At("TAIL"))
  private void renderCustomButton(
    GuiGraphics guiGraphics, int index, int y, int x, int entryWidth, int entryHeight,
    int mouseX, int mouseY, boolean hovered, float tickDelta, CallbackInfo ci
  ) {
    this.buttonX = x + entryWidth - 20;
    this.buttonY = y + 10;

    boolean isHovered = mouseX >= buttonX && mouseX <= buttonX + buttonWidth
      && mouseY >= buttonY && mouseY <= buttonY + buttonHeight;

    guiGraphics.fill(buttonX, buttonY, buttonX + buttonWidth, buttonY + buttonHeight, isHovered ? 0xFFAAAAAA : 0xFF666666);
    guiGraphics.drawCenteredString(
      mc.font,
      Component.literal("X"),
      buttonX + buttonWidth / 2,
      buttonY + 2,
      0xFFFFFFFF
    );
  }

  @Inject(method = "mouseClicked", at = @At("HEAD"), cancellable = true)
  private void onMouseClick(double mouseX, double mouseY, int button, CallbackInfoReturnable<Boolean> cir) {
    if (mouseX >= buttonX && mouseX <= buttonX + buttonWidth
      && mouseY >= buttonY && mouseY <= buttonY + buttonHeight) {

      ServerSelectionList.OnlineServerEntry entry = (ServerSelectionList.OnlineServerEntry) (Object) this;
      String ip = entry.getServerData().ip;

      mc.setScreen(new ServerPlayerListScreen(mc.screen, ip));
      cir.setReturnValue(true);
    }
  }
}