# Pace

Sounds simple, right? Pace is just speed over distance. But there is more to it than meets the eye.

## What is the GAP?
GAP stands for Gradient-Adjusted Pace. In simple words, this pace calculation accounts uphill and downhill movement. The calculation differs depending on your activity type.

### Key Differences Between Running and Cycling
1. **Rolling Resistance Matters:**
   - In running, energy is mostly used to overcome gravity and metabolic costs.
   - In cycling, rolling resistance plays a role, meaning the energy cost of different gradients isn’t as extreme as in running.

2. **Aerodynamic Drag Becomes Significant:**
   - At higher cycling speeds, air resistance (drag) is the dominant force on flat terrain, making speed adjustments different from running.

3. **Power Output Defines Effort, Not Just Metabolic Cost:**
   - Cycling effort is measured in watts (W), and power-to-speed relationships follow a cubic relationship on flat ground due to aerodynamic drag.

### Running
#### Per linear correction

1. **Gradient Conversion**  

   A gradient \( s \)% corresponds to an angle \( \theta \), which can be approximated as:

   > \(\theta \approx \tan^{-1}(s/100)\)


2. **Speed Adjustment Formula**  

   > \(v_{\text{flat}} = v_{\text{incline}} \times \left(1 + \frac{s}{100} \right)\)


##### Example Calculation
Let’s say you’re running **12 km/h** on a **6% incline** (\( s = 0.06 \)):

\(v_{\text{flat}} = 12 \times \left(1 + \frac{6}{100} \right)\)
 
\(v_{\text{flat}} = 12 \times 1.06\)
 
\(v_{\text{flat}} = 12.72 \text{ km/h}\)

If you’re running 12 km/h on a 6% incline, it’s equivalent to running 12.72 km/h on flat ground using the linear correction model. But there is more:

#### The Minetti Metabolic Cost Model

A widely accepted model for running energy cost was developed by Minetti et al. (2002). It estimates the extra energy required to run at different gradients:

> \(C(s) = 155.4s^5 - 30.4s^4 + 5.4s^3 + 0.68s^2 + 1.94s + 3.6\)

**Where**:

- C(s)  is the metabolic energy cost (Joules per kilogram per meter). 
- s  is the gradient as a decimal (e.g., a 5% incline →  s = 0.05 ).

This formula accounts for the fact that running downhill beyond a certain slope (~ -10%) becomes inefficient due to braking forces. The baseline cost for running on flat terrain is 3.6 J/kg/m. 
To estimate equivalent flat pace, we can adjust speed using the metabolic cost ratio:


> \(v_{\text{flat}} = v_{\text{gradient}} \times \frac{C(0)}{C(s)}\)

**Where**:

- \(v_{\text{gradient}}\) is your actual speed on the slope.
- \(v_{\text{flat}}\) is the estimated equivalent flat speed.
- \(C(0) = 3.6\) is the energy cost on flat ground.
- \(C(s)\) is the energy cost at the given gradient.

##### Example Calculation
Let’s say you’re running **12 km/h** on a **6% incline** (\( s = 0.06 \)):

1. **Compute the metabolic cost at 6% slope:**

     \(C(0) = 3.6  J/kg/m (metabolic cost on flat ground).\)

     \(C(0.06) = 5.14  J/kg/m.\)


2. **Compute adjusted speed:**

     \(v_{\text{flat}} = 12 \times \frac{5.14}{3.6}\)

     \(v_{\text{flat}} \approx 17.1 \text{ km/h}\)

**Conclusion:**  
If you’re running 12 km/h on a 6% incline, it’s equivalent to running 17.1 km/h on flat ground in terms of effort. That is quite a difference compared to the linear model with a corrected pace of  12.72 km/h.

#### Why Is the Difference So Big?

- The linear model assumes a fixed percentage effort increase per gradient, which is inaccurate for steep inclines.
- The Minetti model accounts for the true exponential increase in energy demand, meaning the difference in equivalent speed grows significantly as inclines get steeper.

For gradients below ~3%, the linear model is reasonably close, But for steeper inclines, the Minetti model is much more accurate.

### Cycling
The required power output \(P\) for cycling is influenced by:

- Gravitational resistance (on inclines)
- Rolling resistance (constant but varies slightly with slope)
- Aerodynamic drag (depends on speed and wind conditions)

#### Cycling Power-Based Gradient Adjustment

For a steady climb at constant power, the equivalent speed on flat ground is:

> \(v_{\text{flat}} = v_{\text{incline}} \times \left( 1 + \frac{s}{200} \right)\)

For descending, the correction factor is smaller because aerodynamic drag limits downhill speed increases:

> \( v_{\text{flat}} = v_{\text{incline}} \times \frac{1}{1 - \frac{s}{100}}\)

###### Example Calculation
Let’s say you’re cycling 20 km/h on a 6% incline ( s = 6 ).

\(v_{\text{flat}} = 20 \times \left( 1 + \frac{6}{200} \right)\)

\(v_{\text{flat}} = 20 \times 1.03 \)

\(v_{\text{flat}} \approx 20.6 \text{ km/h}\)

Going downhill is not as "easy" to calculate. It increases your effective speed, but - due to air resistance - not linearly.

#### Forces Acting on a Cyclist on a Slope

   The power required to maintain a certain speed while cycling downhill is influenced by:

   - Gravity  \(F_g\) : Helps accelerate the cyclist downhill.
   - Rolling Resistance \(F_r\) : Always resists motion.
   - Aerodynamic Drag  \(F_d\) : Increases quadratically with speed.

   The total force equation on a downhill slope \(( s )\) is:

   > \(F_{\text{net}} = m g \sin(\theta) - F_r - F_d\)

   **Where**:

   - \(m\)  = cyclist mass,
   - \(g\)  = gravity (9.81 m/s²),
   - \(\theta  = slope angle ( \theta \approx \tan^{-1}(s/100) )\),
   - \(F_r = C_r m g \cos(\theta)\)  (rolling resistance, where  \(C_r\)  is the rolling resistance coefficient),
   - \(F_d = \frac{1}{2} C_d A \rho v^2\)  (aerodynamic drag, where  \(C_d\)  is the drag coefficient,  \(A\)  is frontal area,  \(\rho\)  is air density, and  \(v\)  is speed).


#### Nonlinear Speed Adjustment Formula

   Using an approximate power-speed relationship (since power scales with  v^3  on flat ground due to drag):
   > \(v_{\text{flat}} = v_{\text{incline}} \times \sqrt[3]{1 + \frac{|s|}{100}}\)
   
   **Where**:

   - \(|s|\) is the absolute gradient in percent.

##### Example Calculation
Let’s say you are cycling at 40 km/h on a -10% downhill slope. Using the nonlinear formula:


\(v_{\text{flat}} = 40 \times \sqrt[3]{1 + \frac{10}{100}}\)

\(v_{\text{flat}} = 40 \times \sqrt[3]{1.1}\)

\(v_{\text{flat}} \approx 40 \times 1.032\)

\(v_{\text{flat}} \approx 41.3 \text{ km/h}\)

With the linear model, the estimated flat-equivalent speed would have been 44 km/h (too high). With the nonlinear model, it’s more realistic at 41.3 km/h, since aerodynamic drag limits speed gains.

