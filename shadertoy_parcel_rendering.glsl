#define MIN_FLOAT -1e38
#define MAX_FLOAT 1e38


struct ParcelShape {
    vec3 forward;
    vec3 normal;

    // note these are measured in standard deviations, not spatial distances
    float length;
    float width;
    float height;
};

struct Parcel {
    vec3 mean;
    mat3 covariance;
    mat3 inverseCovariance; // TODO: precompute
    float peakDensity;
    vec3 color;
};

struct ParcelSurfacePoint {
    vec3 point;
    float density;
    vec3 normal;
};

//------------------------------------------------------------

vec2 distanceToBounds(
    vec3 cameraPosition,
    vec3 cameraDirection,
    Parcel parcel,
    float sigmas
) {
    mat3 invCov = inverse(parcel.covariance);

    vec3 oc = cameraPosition - parcel.mean;
    vec3 d = normalize(cameraDirection);

    float A = dot(d, invCov * d);
    float B = 2.0 * dot(d, invCov * oc);
    float C = dot(oc, invCov * oc) - sigmas * sigmas;

    float discriminant = B * B - 4.0 * A * C;

    if (discriminant < 0.0)
        return vec2(MIN_FLOAT, MAX_FLOAT);   // no intersection

    float s = sqrt(discriminant);

    float t0 = (-B - s) / (2.0 * A);
    float t1 = (-B + s) / (2.0 * A);

    return vec2(min(t0, t1), max(t0, t1));
}

float parcelDensity(Parcel parcel, vec3 p) {
    vec3 d = p - parcel.mean;
    mat3 invCov = inverse(parcel.covariance);
    float mahalanobisDistanceSquared = dot(d, invCov * d);
    float falloff = exp(-0.5 * mahalanobisDistanceSquared);
    return parcel.peakDensity * falloff;
}

vec2 normalizeScreenCoordinates(vec2 fragCoord) {
    return (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
}

// Gram-Schmidt orthogonalization
// returns v2 stripped of any portion pointing toward v1
vec3 orthogonalize(vec3 v1, vec3 v2) {
    return normalize(v2 - dot(v2, v1) * v1);
}

mat3 covarianceFromShape(ParcelShape shape) {
    vec3 forward = normalize(shape.forward);
    vec3 normal = orthogonalize(forward, shape.normal);
    vec3 right = cross(forward, normal);
    
    float l2 = shape.length * shape.length;
    float w2 = shape.width * shape.width;
    float t2 = shape.height * shape.height;
    
    return
          l2 * outerProduct(forward, forward)
        + w2 * outerProduct(right, right)
        + t2 * outerProduct(normal, normal);
}

vec3 densityGradient(Parcel[3] parcels, vec3 p) {
    vec3 g = vec3(0.0);

    for (int i = 0; i < 3; i++) {
        vec3 d = p - parcels[i].mean;
        float rho = parcelDensity(parcels[i], p);

        g += -rho * (inverse(parcels[i].covariance) * d);
    }

    return g;
}

vec3 normalAtPoint(Parcel[3] parcels, vec3 p) {
    return normalize(-densityGradient(parcels, p));
}


#define MAX_STEPS 2000
#define ISOSURFACE 0.5
#define STEP_SIZE 0.005

float rayMarch(vec3 cameraPosition, vec3 cameraDirection, Parcel[3] parcels) {
    float t = 0.0;
    float maxDensity = 0.0;
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 p = cameraPosition + t * cameraDirection;
        float d = 0.0;
        for(int j = 0; j < 3; j++)
            d += parcelDensity(parcels[j], p);
        maxDensity = max(maxDensity, d);

        if (d > ISOSURFACE)
            return 0.2;

        t += STEP_SIZE;
    }
    return maxDensity;
}

vec3 findSurface(vec3 cameraPosition, vec3 cameraDirection, Parcel[3] parcels) {
    float t = 0.0;
    float maxDensity = 0.0;
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 p = cameraPosition + t * cameraDirection;
        float d = 0.0;
        for(int j = 0; j < 3; j++)
            d += parcelDensity(parcels[j], p);
        maxDensity = max(maxDensity, d);

        if (d > ISOSURFACE)
            return p;

        t += STEP_SIZE;
    }
    return vec3(0);
}

float getLight(vec3 p, vec3 normal) {
    vec3 lightPos = vec3(0, 6, 0);
    //lightPos += 4.0 * vec3(sin(0.1 * iTime), 0.0, cos(0.1 * iTime));
    vec3 lightVector = normalize(lightPos - p);
    float diffuse = clamp(dot(normal, lightVector), 0.0, 1.0);
    return diffuse;
}

Parcel[3] getParcels() {    
    float c = cos(0.5 * iTime);
    float s = sin(0.5 * iTime);
 
    ParcelShape shape1;
    shape1.forward = vec3(0,0,-1);
    shape1.normal = vec3(0,1,0);
    shape1.length = 0.3;
    shape1.width = 0.3;
    shape1.height = 0.02;
    Parcel parcel1;
    parcel1.mean = vec3(-0.8 * c,1,-3);
    parcel1.covariance = covarianceFromShape(shape1);
    parcel1.inverseCovariance = inverse(parcel1.covariance);
    parcel1.peakDensity = 0.8;
    parcel1.color = vec3(0.25, 0.75, 0.35);

    ParcelShape shape2;
    shape2.forward = vec3(0,0,-1);
    shape2.normal = vec3(0,1,0);
    shape2.length = 0.3;
    shape2.width = 0.3;
    shape2.height = 0.02;
    Parcel parcel2;
    parcel2.mean = vec3(0.8 * c,1,-3);
    parcel2.covariance = covarianceFromShape(shape2);
    parcel2.inverseCovariance = inverse(parcel2.covariance);
    parcel2.peakDensity = 0.8;
    parcel2.color = vec3(0.25, 0.75, 0.35);
    
    ParcelShape shape3;
    shape3.forward = vec3(0,0,-1);
    shape3.normal = vec3(0,1,0);
    shape3.length = 0.3;
    shape3.width = 0.02;
    shape3.height = 0.05;
    Parcel parcel3;
    parcel3.mean = vec3(0,1,-3);
    parcel3.covariance = covarianceFromShape(shape3);
    parcel3.inverseCovariance = inverse(parcel3.covariance);
    parcel3.peakDensity = 0.8;
    parcel3.color = vec3(0.25, 0.75, 0.35);

    return Parcel[](parcel1, parcel2, parcel3);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord)
{
    vec2 uv = normalizeScreenCoordinates(fragCoord);
   
    Parcel[3] parcels = getParcels();
    
    vec3 cameraPosition = vec3(.5,1.5,0);
    vec3 cameraDirection = normalize(vec3(uv.x, uv.y, -1));
    float density = rayMarch(cameraPosition, cameraDirection, parcels);
    vec3 surfacePoint = findSurface(cameraPosition, cameraDirection, parcels);
    vec3 normal = normalAtPoint(parcels, surfacePoint);
    float light = getLight(surfacePoint, normal);
    float color = max(light, density);
    if(surfacePoint == vec3(0.0))
        color = density;

    fragColor = vec4(vec3(color * vec3(0.25, 0.75, 0.35)), 1.0);
}