import java.util.Scanner;
import java.util.InputMismatchException;

/**
 * SimpleCalculator provides basic arithmetic functionality (Add, Subtract, Multiply, Divide). 
 * It runs as a console application accepting two numbers and an operator from the user.
 */
public class SimpleCalculator {

    private static double calculate(double num1, double num2, String operator) {
        switch (operator) {
            case "+":
                return num1 + num2;
            case "-":
                return num1 - num2;
            case "*":
                return num1 * num2;
            case "/":
                if (num2 == 0) {
                    // Throw specific exception or handle with Infinity/NaN as needed.
                    throw new ArithmeticException("Error: Division by zero is not allowed.");
                }
                return num1 / num2;
            default:
                throw new IllegalArgumentException("Invalid operator provided. Must be +, -, *, or /.");
        }
    }

    public static void main(String[] args) {
        Scanner scanner = new Scanner(System.in);

        System.out.println("=============================");
        System.out.println("🚀 Simple Console Calculator 🤖");
        System.out.println("-----------------------------");

        try {
            // Get the first number
            System.out.print("Enter the first number: ");
            if (!scanner.hasNextDouble()) {
                System.err.println("Invalid input. Please enter a valid number.");
                return;
            }
            double num1 = scanner.nextDouble();

            // Get the operator
            System.out.print("Enter the operation (+, -, *, /): ");
            String operator = scanner.next();

            // Get the second number
            System.out.print("Enter the second number: ");
             if (!scanner.hasNextDouble()) {
                System.err.println("Invalid input. Please enter a valid number.");
                return;
            }
            double num2 = scanner.nextDouble();

            // Calculate and display result
            try {
                double result = calculate(num1, num2, operator);
                System.out.println("\n=============================");
                System.out.printf("Result: %.2f %s %.2f = %.2f%n", num1, operator, num2, result);
                System.out.println("=============================");

            } catch (ArithmeticException e) {
                System.err.println("\n❌ Calculation Error: " + e.getMessage());
            } catch (IllegalArgumentException e) {
                 System.err.println("\n❌ Invalid Operation: " + e.getMessage());
            }


        } catch (Exception e) {
            // Catch general IO or runtime errors
            System.err.println("\nAn unexpected error occurred: " + e.getMessage());
        } finally {
            scanner.close();
        }
    }
}