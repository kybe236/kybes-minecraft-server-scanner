package de.kybe.screens;

import de.kybe.module.modules.Scanner;
import de.kybe.utils.CrackedUtils;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.ObjectSelectionList;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jetbrains.annotations.NotNull;

import java.util.List;

public class ServerPlayerListScreen extends Screen {
  private final Screen parent;
  private final String serverIp;
  private PlayerListWidget playerListWidget;


  public ServerPlayerListScreen(Screen parent, String serverIp) {
    super(Component.literal("Server Info"));
    this.parent = parent;
    this.serverIp = serverIp;
  }

  @Override
  protected void init() {
    super.init();

    List<String> players = Scanner.INSTANCE.getPlayers(serverIp);
    players.sort(String::compareToIgnoreCase);

    int top = 80;
    int bottom = this.height - 40;
    playerListWidget = new PlayerListWidget(this.minecraft, this.width, this.height - 30, 30, this.minecraft.font.lineHeight * 2, 40, players);
    this.addWidget(playerListWidget);
  }


  @Override
  public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
    playerListWidget.render(graphics, mouseX, mouseY, delta);

    graphics.drawString(this.font, "Server IP: " + serverIp, 10, 10, 0xFFFFFFFF);

    super.render(graphics, mouseX, mouseY, delta);
  }

  @Override
  public void onClose() {
    if (this.minecraft == null || this.parent == null) return;
    this.minecraft.setScreen(parent);
  }

  // Inner class for player list widget
  private class PlayerListWidget extends ObjectSelectionList<PlayerListWidget.PlayerEntry> {

    public PlayerListWidget(net.minecraft.client.Minecraft minecraft, int width, int height, int top, int itemHeight, int headerHeight, List<String> players) {
      super(minecraft, width, height, top, itemHeight, itemHeight);
      for (String player : players) {
        this.addEntry(new PlayerEntry(player));
      }
    }

    @Override
    protected int scrollBarX() {
      return this.width - 6;
    }

    @Override
    public int getRowWidth() {
      return 200;
    }

    // Represents one player in the list
    public class PlayerEntry extends ObjectSelectionList.Entry<PlayerEntry> {
      private final String playerName;

      public PlayerEntry(String playerName) {
        this.playerName = playerName;
      }

      @Override
      public void render(GuiGraphics graphics, int index, int y, int x, int listWidth, int itemHeight, int mouseX, int mouseY, boolean hovered, float delta) {
        int nameWidth = font.width(playerName);
        int centerX = x + (listWidth - nameWidth) / 2;
        graphics.drawString(font, playerName, centerX, y + 5, 0xFFFFFFFF);
      }

      @Override
      public @NotNull Component getNarration() {
        return Component.literal(playerName);
      }

      @Override
      public boolean mouseClicked(double mouseX, double mouseY, int button) {
        if (button == 0) {
          System.out.println("Logging in as: " + playerName);

          CrackedUtils.login(playerName);

          return true;
        }
        return false;
      }
    }
  }
}