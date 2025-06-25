package de.kybe.utils;

import net.minecraft.network.chat.Component;

import java.awt.*;

import static de.kybe.Constants.mc;

public class ChatUtils {
  public static void print(String message) {
    ChatUtils.print(Component.literal(message));
  }

  public static void print(Component message) {
    Component bracketLeft = Component.literal("[").withColor(new Color(200, 0, 0).getRGB());
    Component bracketRight = Component.literal("] ").withColor(new Color(200, 0, 0).getRGB());
    Component kybe = Component.literal("KYBE").withColor(new Color(139, 0, 0).getRGB());
    Component prefix = bracketLeft.copy().append(kybe).append(bracketRight);
    Component finalMessage = prefix.copy().append(message).withColor(Color.WHITE.getRGB());
    printRaw(finalMessage);
  }

  public static void printRaw(Component message) {
    mc.gui.getChat().addMessage(message);
  }

  @SuppressWarnings("unused")
  public static void printRaw(String  message) {
    mc.gui.getChat().addMessage(Component.literal(message));
  }
}
